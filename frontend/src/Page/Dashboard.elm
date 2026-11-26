module Page.Dashboard exposing (Model, Msg, Tab(..), init, subscriptions, switchTab, update, view)

{-| The signed-in app: WhatsApp bots, prepaid credits, settings.

Each tab owns one `RemoteData` field and fetches on first visit. Tabs are
real routes, so they're linkable and the back button works.

-}

import Api
import BotEditor
import Browser.Navigation as Nav
import Format
import Html exposing (Html, a, div, h1, h2, li, p, section, span, text, ul)
import Html.Attributes exposing (class, classList)
import Html.Events exposing (onClick)
import Json.Encode as E
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Ui


type Tab
    = Bots
    | Billing
    | Settings


type alias Model =
    { tab : Tab
    , channels : Api.Data Api.WhatsAppView
    , billing : Api.Data Api.BillingView
    , settings : Api.Data Api.SettingsView
    , settingsForm : SettingsForm
    , editing : Maybe { id : String, reply : Api.ReplyConfig, waitText : String }
    , confirmDisconnect : Maybe String
    , creditsToBuy : Int
    , deleteConfirm : String
    , saving : Bool
    , connectingMeta : Bool
    , notice : Maybe ( String, String )
    }


type alias SettingsForm =
    { emails : List String
    , idleGap : String
    , history : String
    , handoffEmail : Bool
    , locale : String
    , currency : String
    }


emptySettingsForm : SettingsForm
emptySettingsForm =
    { emails = [ "" ]
    , idleGap = ""
    , history = ""
    , handoffEmail = True
    , locale = ""
    , currency = ""
    }


formFromSettings : Api.SettingsView -> SettingsForm
formFromSettings s =
    let
        emails =
            if List.isEmpty s.notifications.emails then
                if String.isEmpty (String.trim s.email) then
                    [ "" ]

                else
                    [ s.email ]

            else
                s.notifications.emails
    in
    { emails = emails
    , idleGap = maybeIntToString s.conversation.idleGapMins
    , history = maybeIntToString s.conversation.maxHistoryMessages
    , handoffEmail = s.notifications.handoffEmail
    , locale = s.locale
    , currency = s.currency
    }


maybeIntToString : Maybe Int -> String
maybeIntToString =
    Maybe.map String.fromInt >> Maybe.withDefault ""


init : Tab -> Maybe String -> ( Model, Cmd Msg )
init tab initialError =
    let
        model =
            { tab = tab
            , channels = NotAsked
            , billing = NotAsked
            , settings = NotAsked
            , settingsForm = emptySettingsForm
            , editing = Nothing
            , confirmDisconnect = Nothing
            , creditsToBuy = 0
            , deleteConfirm = ""
            , saving = False
            , connectingMeta = False
            , notice = Maybe.map (\msg -> ( "error", msg )) initialError
            }
    in
    ( model, fetchFor tab model )


fetchFor : Tab -> Model -> Cmd Msg
fetchFor tab model =
    case tab of
        Bots ->
            Cmd.batch
                [ if RemoteData.isNotAsked model.channels then
                    Api.getWhatsApp GotChannels

                  else
                    Cmd.none
                , if RemoteData.isNotAsked model.billing then
                    Api.getBilling GotBilling

                  else
                    Cmd.none
                ]

        Billing ->
            if RemoteData.isNotAsked model.billing then
                Api.getBilling GotBilling

            else
                Cmd.none

        Settings ->
            if RemoteData.isNotAsked model.settings then
                Api.getSettings GotSettings

            else
                Cmd.none


switchTab : Tab -> Model -> ( Model, Cmd Msg )
switchTab tab model =
    update (SwitchedTo tab) model



-- UPDATE


type Msg
    = SwitchedTo Tab
    | GotChannels (Api.Data Api.WhatsAppView)
    | RefreshedChannels (Api.Data Api.WhatsAppView)
    | StartMetaSignup Api.MetaSignup
    | MetaSignupCame E.Value
    | GotBilling (Api.Data Api.BillingView)
    | EditReply String Api.ReplyConfig
    | SetEditReply Api.ReplyConfig
    | SetWaitText String
    | CancelEdit
    | SaveReply
    | ToggleBotEnabled Api.WhatsAppAccount
    | SavedReply (Api.Data Api.WhatsAppAccount)
    | AskDisconnect String
    | CancelDisconnect
    | ConfirmDisconnect String
    | DisconnectedWhatsApp (Api.Data ())
    | SetCredits String
    | BuyCredits
    | GotOrder (Api.Data Api.Order)
    | PaymentCame Ports.PaymentOutcome
    | GotSettings (Api.Data Api.SettingsView)
    | SetNotificationEmail Int String
    | AddNotificationEmail
    | RemoveNotificationEmail Int
    | SetTiming (SettingsForm -> SettingsForm)
    | SaveTiming
    | SetHandoffEmail Bool
    | SaveNotifications
    | SetLocaleField String
    | SetCurrencyField String
    | SaveLocale
    | SavedSettings (Api.Data Api.SettingsView)
    | SetDeleteConfirm String
    | CloseAccount
    | AccountClosed (Api.Data ())


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SwitchedTo tab ->
            ( { model | tab = tab, notice = Nothing, confirmDisconnect = Nothing }, fetchFor tab model )

        GotChannels result ->
            ( { model | channels = result, connectingMeta = False }, Cmd.none )

        RefreshedChannels result ->
            case result of
                Success data ->
                    ( { model | channels = Success data }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        StartMetaSignup signup ->
            ( { model | connectingMeta = True, notice = Nothing }
            , Ports.openMetaSignup
                { appId = signup.appId
                , configId = signup.configId
                , state = signup.state
                }
            )

        MetaSignupCame raw ->
            case Ports.decodeMetaSignupOutcome raw of
                Ports.MetaSignupOk ->
                    ( { model
                        | connectingMeta = False
                        , notice = Just ( "success", "WhatsApp Business number connected." )
                      }
                    , Api.getWhatsApp GotChannels
                    )

                Ports.MetaSignupCancelled ->
                    ( { model
                        | connectingMeta = False
                        , notice = Just ( "info", "WhatsApp signup was cancelled." )
                      }
                    , Api.getWhatsApp RefreshedChannels
                    )

                Ports.MetaSignupError errMsg ->
                    ( { model
                        | connectingMeta = False
                        , notice = Just ( "error", errMsg )
                      }
                    , Api.getWhatsApp RefreshedChannels
                    )

        GotBilling result ->
            let
                credits =
                    case result of
                        Success billing ->
                            clamp billing.pricing.minCredits billing.pricing.maxCredits (max model.creditsToBuy billing.pricing.minCredits)

                        _ ->
                            model.creditsToBuy
            in
            ( { model | billing = result, creditsToBuy = credits }, Cmd.none )

        EditReply id reply ->
            ( { model
                | editing = Just { id = id, reply = reply, waitText = String.fromInt reply.waitSeconds }
                , confirmDisconnect = Nothing
                , notice = Nothing
              }
            , Cmd.none
            )

        SetEditReply reply ->
            ( { model | editing = Maybe.map (\e -> { e | reply = reply }) model.editing }
            , Cmd.none
            )

        SetWaitText value ->
            ( { model | editing = Maybe.map (\e -> { e | waitText = value }) model.editing }
            , Cmd.none
            )

        CancelEdit ->
            ( { model | editing = Nothing }, Cmd.none )

        SaveReply ->
            case ( model.editing, model.channels ) of
                ( Just edit, Success { accounts } ) ->
                    case String.toInt (String.trim edit.waitText) of
                        Just waitSecs ->
                            if waitSecs < 0 || waitSecs > 30 then
                                ( { model
                                    | notice =
                                        Just ( "error", "Burst wait window must be between 0 and 30 seconds." )
                                  }
                                , Cmd.none
                                )

                            else
                                case BotEditor.validate edit.reply of
                                    Just err ->
                                        ( { model | notice = Just ( "error", err ) }, Cmd.none )

                                    Nothing ->
                                        let
                                            name =
                                                accounts
                                                    |> List.filter (\a -> a.id == edit.id)
                                                    |> List.head
                                                    |> Maybe.map .name
                                                    |> Maybe.withDefault ""
                                        in
                                        ( { model | saving = True, notice = Nothing }
                                        , Api.updateWhatsApp edit.id
                                            { name = name
                                            , enabled = edit.reply.enabled
                                            , botPrompt = edit.reply.botPrompt
                                            , rules = edit.reply.rules
                                            , waitSeconds = waitSecs
                                            }
                                            SavedReply
                                        )

                        Nothing ->
                            ( { model
                                | notice =
                                    Just ( "error", "Burst wait window must be a whole number from 0 to 30." )
                              }
                            , Cmd.none
                            )

                _ ->
                    ( model, Cmd.none )

        ToggleBotEnabled account ->
            ( { model | saving = True, notice = Nothing }
            , Api.updateWhatsApp account.id
                { name = account.name
                , enabled = not account.reply.enabled
                , botPrompt = account.reply.botPrompt
                , rules = account.reply.rules
                , waitSeconds = account.reply.waitSeconds
                }
                SavedReply
            )

        SavedReply result ->
            case result of
                Success account ->
                    ( { model
                        | saving = False
                        , editing = Nothing
                        , notice = Just ( "success", "Saved." )
                        , channels =
                            RemoteData.map
                                (\c ->
                                    { c
                                        | accounts =
                                            List.map
                                                (\a ->
                                                    if a.id == account.id then
                                                        account

                                                    else
                                                        a
                                                )
                                                c.accounts
                                    }
                                )
                                model.channels
                      }
                    , Cmd.none
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        AskDisconnect id ->
            ( { model | confirmDisconnect = Just id, notice = Nothing }, Cmd.none )

        CancelDisconnect ->
            ( { model | confirmDisconnect = Nothing }, Cmd.none )

        ConfirmDisconnect id ->
            ( { model | saving = True, notice = Nothing }
            , Api.deleteWhatsApp id DisconnectedWhatsApp
            )

        DisconnectedWhatsApp result ->
            case result of
                Success () ->
                    ( { model
                        | saving = False
                        , confirmDisconnect = Nothing
                        , editing = Nothing
                        , notice = Just ( "success", "WhatsApp number disconnected." )
                      }
                    , Api.getWhatsApp GotChannels
                    )

                Failure err ->
                    ( { model
                        | saving = False
                        , confirmDisconnect = Nothing
                        , notice = Just ( "error", Api.errorMessage err )
                      }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        SetCredits raw ->
            ( { model | creditsToBuy = String.toInt raw |> Maybe.withDefault model.creditsToBuy }
            , Cmd.none
            )

        BuyCredits ->
            ( { model | saving = True, notice = Nothing }
            , Api.startCheckout model.creditsToBuy GotOrder
            )

        GotOrder result ->
            case result of
                Success order ->
                    ( { model | saving = False }
                    , Ports.openCheckout
                        (E.object
                            [ ( "order_id", E.string order.orderId )
                            , ( "amount", E.int order.amount )
                            , ( "currency", E.string order.currency )
                            , ( "key_id", E.string order.keyId )
                            ]
                        )
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        PaymentCame outcome ->
            if String.isEmpty outcome.signature then
                ( { model
                    | saving = False
                    , notice =
                        Just
                            ( "error"
                            , if String.isEmpty outcome.error then
                                "Payment cancelled."

                              else
                                outcome.error
                            )
                  }
                , Cmd.none
                )

            else
                ( { model
                    | saving = False
                    , notice =
                        Just
                            ( "success"
                            , "Payment received. Your credit balance updates within a few seconds."
                            )
                  }
                , Api.getBilling GotBilling
                )

        GotSettings result ->
            ( { model
                | settings = result
                , settingsForm =
                    case result of
                        Success settings ->
                            formFromSettings settings

                        _ ->
                            model.settingsForm
              }
            , Cmd.none
            )

        SetNotificationEmail idx val ->
            let
                form =
                    model.settingsForm

                nextEmails =
                    List.indexedMap
                        (\i e ->
                            if i == idx then
                                val

                            else
                                e
                        )
                        form.emails
            in
            ( { model | settingsForm = { form | emails = nextEmails } }, Cmd.none )

        AddNotificationEmail ->
            let
                form =
                    model.settingsForm
            in
            ( { model | settingsForm = { form | emails = form.emails ++ [ "" ] } }, Cmd.none )

        RemoveNotificationEmail idx ->
            let
                form =
                    model.settingsForm

                remaining =
                    List.indexedMap Tuple.pair form.emails
                        |> List.filter (\( i, _ ) -> i /= idx)
                        |> List.map Tuple.second

                nextEmails =
                    if List.isEmpty remaining then
                        [ "" ]

                    else
                        remaining
            in
            ( { model | settingsForm = { form | emails = nextEmails } }, Cmd.none )

        SetTiming change ->
            ( { model | settingsForm = change model.settingsForm }, Cmd.none )

        SaveTiming ->
            case model.settings of
                Success settings ->
                    let
                        form =
                            model.settingsForm

                        read txt stored =
                            if String.trim txt == "" then
                                Nothing

                            else
                                case String.toInt (String.trim txt) of
                                    Just n ->
                                        Just n

                                    Nothing ->
                                        stored

                        conversation =
                            settings.conversation
                    in
                    ( { model | saving = True, notice = Nothing }
                    , Api.saveConversation
                        { conversation
                            | idleGapMins = read form.idleGap conversation.idleGapMins
                            , maxHistoryMessages = read form.history conversation.maxHistoryMessages
                        }
                        SavedSettings
                    )

                _ ->
                    ( model, Cmd.none )

        SetHandoffEmail checked ->
            let
                form =
                    model.settingsForm
            in
            ( { model | settingsForm = { form | handoffEmail = checked } }, Cmd.none )

        SaveNotifications ->
            let
                cleanedEmails =
                    model.settingsForm.emails
                        |> List.map String.trim
                        |> List.filter (not << String.isEmpty)
            in
            ( { model | saving = True, notice = Nothing }
            , Api.saveNotifications
                { handoffEmail = model.settingsForm.handoffEmail
                , emails = cleanedEmails
                }
                SavedSettings
            )

        SetLocaleField tag ->
            let
                form =
                    model.settingsForm
            in
            ( { model | settingsForm = { form | locale = tag } }, Cmd.none )

        SetCurrencyField code ->
            let
                form =
                    model.settingsForm
            in
            ( { model | settingsForm = { form | currency = code } }, Cmd.none )

        SaveLocale ->
            ( { model | saving = True, notice = Nothing }
            , Api.saveLocale
                { locale = model.settingsForm.locale
                , currency = model.settingsForm.currency
                }
                SavedSettings
            )

        SavedSettings result ->
            case result of
                Success settings ->
                    ( { model
                        | saving = False
                        , settings = Success settings
                        , settingsForm = formFromSettings settings
                        , notice = Just ( "success", "Saved." )
                      }
                    , Cmd.none
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        SetDeleteConfirm value ->
            ( { model | deleteConfirm = value, notice = Nothing }, Cmd.none )

        CloseAccount ->
            ( { model | saving = True, notice = Nothing }
            , Api.deleteAccount (String.trim model.deleteConfirm) AccountClosed
            )

        AccountClosed result ->
            case result of
                Success () ->
                    ( { model | saving = False }, Nav.load "/" )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )


subscriptions : Sub Msg
subscriptions =
    Sub.batch
        [ Ports.paymentOutcome PaymentCame
        , Ports.metaSignupOutcome MetaSignupCame
        ]



-- VIEW


view : Api.Session -> Model -> Html Msg
view session model =
    div [ class "dashboard" ]
        [ h1 [] [ text (greeting session) ]
        , safetyNotice session
        , tabs model.tab
        , case model.notice of
            Just ( kind, message ) ->
                Ui.banner kind message

            Nothing ->
                text ""
        , case model.tab of
            Bots ->
                botsTab model

            Billing ->
                billingTab session model

            Settings ->
                settingsTab session model
        ]


safetyNotice : Api.Session -> Html Msg
safetyNotice session =
    if session.safetyStatus == "rejected" then
        Ui.banner "error"
            (Maybe.withDefault
                "AI replies are paused because your account did not pass the safety check. Rule-matched replies still go out."
                session.safetyReason
            )

    else if session.safetyStatus == "pending" || not session.aiReady then
        Ui.banner "info"
            "AI replies are paused while safety checks complete. Rule-matched replies still go out."

    else
        text ""


greeting : Api.Session -> String
greeting session =
    case session.name of
        Just name ->
            "Hello, " ++ name

        Nothing ->
            "Your dashboard"


tabs : Tab -> Html Msg
tabs current =
    ul [ class "tabs" ]
        (List.map
            (\( tab, route, label_ ) ->
                li []
                    [ a
                        [ Route.href route
                        , classList [ ( "tab", True ), ( "is-active", tab == current ) ]
                        ]
                        [ text label_ ]
                    ]
            )
            [ ( Bots, Route.Dashboard, "Bots" )
            , ( Billing, Route.DashboardBilling, "Credits" )
            , ( Settings, Route.DashboardSettings, "Settings" )
            ]
        )


replyPill : Bool -> Html msg
replyPill enabled =
    span
        [ classList
            [ ( "pill", True )
            , ( "pill-ok", enabled )
            , ( "pill-warn", not enabled )
            ]
        ]
        [ text
            (if enabled then
                "Active"

             else
                "Paused"
            )
        ]



-- BOTS TAB (WHATSAPP)


botsTab : Model -> Html Msg
botsTab model =
    let
        zeroCreditsBanner =
            case model.billing of
                Success billing ->
                    if billing.metered && billing.balance == 0 then
                        Ui.banner "info"
                            "You have 0 credits left. Buy credits in the Credits tab to start sending WhatsApp auto-replies."

                    else
                        text ""

                _ ->
                    text ""
    in
    section []
        [ zeroCreditsBanner
        , Ui.remote model.channels (whatsappSection model)
        ]


whatsappSection : Model -> Api.WhatsAppView -> Html Msg
whatsappSection model data =
    div []
        [ if List.isEmpty data.accounts then
            Ui.colorCard "color-bar-blue"
                [ h2 [] [ text "WhatsApp Business" ]
                , p [ class "muted" ]
                    [ text "Connect your WhatsApp Business number via Meta Embedded Signup to start replying automatically." ]
                , connectButton model True data.signup
                ]

          else
            div []
                (List.map (accountCard model data.botPresets) data.accounts
                    ++ [ case data.signup of
                            Just _ ->
                                Ui.colorCard "color-bar-blue"
                                    [ h2 [] [ text "Add another number" ]
                                    , p [ class "muted" ]
                                        [ text "Connect an additional WhatsApp Business number through Meta Embedded Signup." ]
                                    , connectButton model False data.signup
                                    ]

                            Nothing ->
                                text ""
                       ]
                )
        ]


connectButton : Model -> Bool -> Maybe Api.MetaSignup -> Html Msg
connectButton model isPrimary maybeSignup =
    case maybeSignup of
        Just signup ->
            div [ class "card-actions" ]
                [ Ui.button
                    { label = "Connect WhatsApp number"
                    , onClick = StartMetaSignup signup
                    , primary = isPrimary
                    , busy = model.connectingMeta || model.saving
                    }
                ]

        Nothing ->
            Ui.banner "info" "WhatsApp signup isn't configured on this deployment."


accountCard : Model -> List Api.BotPreset -> Api.WhatsAppAccount -> Html Msg
accountCard model botPresets account =
    let
        editing =
            case model.editing of
                Just e ->
                    if e.id == account.id then
                        Just e

                    else
                        Nothing

                Nothing ->
                    Nothing
    in
    Ui.colorCard "color-bar-green"
        [ div [ class "bot-card-head" ]
            [ h2 [] [ text account.name ]
            , span [ class "connected-phone" ] [ text account.displayPhoneNumber ]
            , replyPill account.reply.enabled
            ]
        , case editing of
            Just edit ->
                div []
                    [ Ui.toggle
                        { id = "reply-enabled-" ++ account.id
                        , label = "Reply automatically to new messages"
                        , checked = edit.reply.enabled
                        , onCheck = \v -> SetEditReply (setEnabled v edit.reply)
                        }
                    , BotEditor.view
                        { idPrefix = "wa-" ++ account.id
                        , presets = List.map toBotEditorPreset botPresets
                        , config = toBotEditorConfig edit.reply
                        , onSelectPreset = \preset -> SetEditReply (applyPreset preset edit.reply)
                        , onChange = \cfg -> SetEditReply (applyBotConfig cfg edit.reply)
                        }
                    , Ui.field
                        { id = "wait-" ++ account.id
                        , label = "Burst wait window (seconds, 0–30)"
                        , value = edit.waitText
                        , hint = "Waits 0–30 seconds for a customer to finish typing a burst of short messages before sending one reply. 0 replies immediately."
                        , required = False
                        , onInput = SetWaitText
                        }
                    , div [ class "card-actions" ]
                        [ Ui.button { label = "Cancel", onClick = CancelEdit, primary = False, busy = False }
                        , Ui.button { label = "Save", onClick = SaveReply, primary = True, busy = model.saving }
                        ]
                    ]

            Nothing ->
                div []
                    [ if String.isEmpty (String.trim account.reply.botPrompt) then
                        text ""

                      else
                        p [ class "bot-card-prompt" ] [ text account.reply.botPrompt ]
                    , if List.isEmpty account.reply.rules then
                        text ""

                      else
                        ul [ class "bot-card-rules" ]
                            (List.map
                                (\rule ->
                                    li [ class "bot-card-rule" ]
                                        [ span [ class "bot-card-rule-match" ] [ text rule.matchRule ]
                                        , span [ class "bot-card-rule-reply" ] [ text rule.replyText ]
                                        ]
                                )
                                account.reply.rules
                            )
                    , if model.confirmDisconnect == Just account.id then
                        div []
                            [ Ui.banner "error" "Disconnect this WhatsApp number? replybots will stop replying on it immediately."
                            , div [ class "card-actions" ]
                                [ Html.button
                                    [ class "btn btn-danger"
                                    , Html.Attributes.type_ "button"
                                    , Html.Attributes.disabled model.saving
                                    , onClick (ConfirmDisconnect account.id)
                                    ]
                                    [ text
                                        (if model.saving then
                                            "Disconnecting…"

                                         else
                                            "Yes, disconnect"
                                        )
                                    ]
                                , Ui.button
                                    { label = "Cancel"
                                    , onClick = CancelDisconnect
                                    , primary = False
                                    , busy = False
                                    }
                                ]
                            ]

                      else
                        div [ class "card-actions" ]
                            [ Ui.button
                                { label = "Edit bot"
                                , onClick = EditReply account.id account.reply
                                , primary = False
                                , busy = False
                                }
                            , Ui.button
                                { label =
                                    if account.reply.enabled then
                                        "Pause"

                                    else
                                        "Resume"
                                , onClick = ToggleBotEnabled account
                                , primary = False
                                , busy = model.saving
                                }
                            , Html.button
                                [ class "btn btn-danger"
                                , Html.Attributes.type_ "button"
                                , Html.Attributes.disabled model.saving
                                , onClick (AskDisconnect account.id)
                                ]
                                [ text "Disconnect" ]
                            ]
                    ]
        ]


setEnabled : Bool -> Api.ReplyConfig -> Api.ReplyConfig
setEnabled enabled reply =
    { reply | enabled = enabled }


toBotEditorPreset : Api.BotPreset -> BotEditor.Preset
toBotEditorPreset preset =
    { id = preset.id
    , label = preset.label
    , tag = preset.tag
    , subtitle = ""
    , prompt = preset.prompt
    , rules = preset.rules
    }


toBotEditorConfig : Api.ReplyConfig -> BotEditor.Config
toBotEditorConfig reply =
    { botPrompt = reply.botPrompt
    , rules = reply.rules
    }


applyPreset : BotEditor.Preset -> Api.ReplyConfig -> Api.ReplyConfig
applyPreset preset reply =
    { reply
        | botPrompt = preset.prompt
        , rules = preset.rules
    }


applyBotConfig : BotEditor.Config -> Api.ReplyConfig -> Api.ReplyConfig
applyBotConfig cfg reply =
    { reply
        | botPrompt = cfg.botPrompt
        , rules = cfg.rules
    }



-- CREDITS TAB


billingTab : Api.Session -> Model -> Html Msg
billingTab session model =
    Ui.remote model.billing <|
        \billing ->
            let
                unitPrice =
                    Api.unitPriceFor billing.currency billing.pricing
            in
            section []
                [ Ui.colorCard "color-bar-amber"
                    [ h2 [] [ text "Balance" ]
                    , p [ class "balance" ]
                        [ text (Format.count session.locale billing.balance ++ " credits") ]
                    , p [ class "muted" ]
                        [ text
                            (Format.count session.locale billing.repliesUsed
                                ++ " replies used so far · 1 credit = 1 reply."
                            )
                        ]
                    , if billing.metered && billing.balance == 0 then
                        Ui.banner "info"
                            "You have 0 credits left. Buy credits to start sending WhatsApp auto-replies."

                      else
                        text ""
                    ]
                , if billing.metered then
                    Ui.colorCard "color-bar-blue"
                        [ h2 [] [ text "Buy credits" ]
                        , p []
                            [ text
                                (Format.milliRate session.locale billing.currency unitPrice
                                    ++ " per reply."
                                )
                            ]
                        , div [ class "credit-picker" ]
                            [ Html.input
                                [ Html.Attributes.type_ "range"
                                , Html.Attributes.min (String.fromInt billing.pricing.minCredits)
                                , Html.Attributes.max (String.fromInt billing.pricing.maxCredits)
                                , Html.Attributes.step (String.fromInt billing.pricing.minCredits)
                                , Html.Attributes.value (String.fromInt model.creditsToBuy)
                                , Html.Events.onInput SetCredits
                                , class "credit-slider"
                                , Html.Attributes.attribute "aria-label" "Number of credits to buy"
                                ]
                                []
                            , Html.input
                                [ Html.Attributes.type_ "number"
                                , Html.Attributes.min (String.fromInt billing.pricing.minCredits)
                                , Html.Attributes.max (String.fromInt billing.pricing.maxCredits)
                                , Html.Attributes.step (String.fromInt billing.pricing.minCredits)
                                , Html.Attributes.value (String.fromInt model.creditsToBuy)
                                , Html.Events.onInput SetCredits
                                , class "credit-input"
                                , Html.Attributes.attribute "aria-label" "Credits to buy"
                                ]
                                []
                            ]
                        , p [ class "credit-total" ]
                            [ text
                                (Format.count session.locale model.creditsToBuy
                                    ++ " credits for "
                                    ++ Format.money session.locale
                                        billing.currency
                                        (totalFor model.creditsToBuy unitPrice)
                                )
                            ]
                        , Ui.button
                            { label = "Buy credits"
                            , onClick = BuyCredits
                            , primary = True
                            , busy = model.saving
                            }
                        ]

                  else
                    Ui.card
                        [ h2 [] [ text "Complimentary account" ]
                        , p [] [ text "You're not charged for replies, and there's nothing to buy." ]
                        ]
                , if List.isEmpty billing.ledger then
                    text ""

                  else
                    Ui.colorCard "color-bar-violet"
                        [ h2 [] [ text "Invoices" ]
                        , ul [ class "ledger" ]
                            (List.map
                                (\credit ->
                                    li []
                                        [ span []
                                            [ text
                                                (Format.count session.locale credit.amount
                                                    ++ " credits"
                                                )
                                            ]
                                        , if String.isEmpty credit.grantedAt then
                                            text ""

                                          else
                                            span [ class "muted" ] [ text (String.left 10 credit.grantedAt) ]
                                        , a
                                            [ Html.Attributes.href ("/api/billing/invoices/" ++ credit.id)
                                            , Html.Attributes.download ("invoice-" ++ credit.id ++ ".html")
                                            , class "btn btn-secondary"
                                            ]
                                            [ text "Download invoice" ]
                                        ]
                                )
                                billing.ledger
                            )
                        ]
                ]


totalFor : Int -> Int -> Int
totalFor credits milliPrice =
    (credits * milliPrice + 500) // 1000



-- SETTINGS TAB


settingsTab : Api.Session -> Model -> Html Msg
settingsTab session model =
    let
        accountEmail =
            case model.settings of
                Success s ->
                    s.email

                _ ->
                    session.email

        hasEmail =
            not (String.isEmpty (String.trim accountEmail))

        confirmTarget =
            if hasEmail then
                accountEmail

            else
                "DELETE"

        confirmed =
            if hasEmail then
                String.toLower (String.trim model.deleteConfirm)
                    == String.toLower accountEmail
                    || String.toUpper (String.trim model.deleteConfirm)
                    == "DELETE"

            else
                String.toUpper (String.trim model.deleteConfirm) == "DELETE"
    in
    section []
        [ Ui.remote model.settings
            (\settings ->
                div []
                    [ timingCard model settings
                    , notificationsCard model
                    , localeCard model
                    ]
            )
        , Ui.card
            [ h2 [] [ text "Close your account" ]
            , p []
                [ text "Deletes your settings, your connected WhatsApp numbers, and your message metadata. Payment records are kept with your account identifier removed, because tax and dispute rules require it. This cannot be undone." ]
            , Ui.field
                { id = "confirm_email"
                , label = "Type " ++ confirmTarget ++ " to confirm"
                , value = model.deleteConfirm
                , hint = ""
                , required = False
                , onInput = SetDeleteConfirm
                }
            , div [ class "card-actions" ]
                [ Html.button
                    [ class "btn btn-danger"
                    , Html.Attributes.type_ "button"
                    , Html.Attributes.disabled (not confirmed || model.saving)
                    , onClick CloseAccount
                    ]
                    [ text
                        (if model.saving then
                            "Closing…"

                         else
                            "Close my account permanently"
                        )
                    ]
                ]
            ]
        ]


timingCard : Model -> Api.SettingsView -> Html Msg
timingCard model settings =
    let
        c =
            settings.conversation

        form =
            model.settingsForm
    in
    Ui.colorCard "color-bar-green"
        [ h2 [] [ text "Conversation timing" ]
        , p [ class "muted" ]
            [ text "How replybots decides where one conversation ends and the next begins." ]
        , Ui.field
            { id = "idle_gap_mins"
            , label = "Minutes of silence before a new conversation"
            , value = form.idleGap
            , hint =
                "After this long without a message, the next message starts a new conversation with no earlier context. Default is "
                    ++ String.fromInt c.defaults.idleGapMins
                    ++ " minutes. Leave empty to use the default."
            , required = False
            , onInput = \v -> SetTiming (\f -> { f | idleGap = v })
            }
        , Ui.field
            { id = "max_history_messages"
            , label = "Recent messages sent to the AI"
            , value = form.history
            , hint =
                "How many recent messages the AI reads before replying. Default is "
                    ++ String.fromInt c.defaults.maxHistoryMessages
                    ++ ". Leave empty to use the default."
            , required = False
            , onInput = \v -> SetTiming (\f -> { f | history = v })
            }
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Save timing"
                , onClick = SaveTiming
                , primary = True
                , busy = model.saving
                }
            ]
        ]


notificationsCard : Model -> Html Msg
notificationsCard model =
    let
        emailRows =
            List.indexedMap
                (\idx addr ->
                    let
                        fieldId =
                            "notification_email_" ++ String.fromInt idx
                    in
                    div [ class "email-row" ]
                        [ Ui.field
                            { id = fieldId
                            , label =
                                if idx == 0 then
                                    "Notification email"

                                else
                                    "Additional email #" ++ String.fromInt (idx + 1)
                            , value = addr
                            , hint = ""
                            , required = False
                            , onInput = SetNotificationEmail idx
                            }
                        , Html.button
                            [ class "btn btn-ghost"
                            , Html.Attributes.type_ "button"
                            , onClick (RemoveNotificationEmail idx)
                            ]
                            [ text "Remove" ]
                        ]
                )
                model.settingsForm.emails
    in
    Ui.colorCard "color-bar-violet"
        [ h2 [] [ text "Notifications" ]
        , Ui.toggle
            { id = "handoff_email"
            , label = "Email me when a customer needs a human"
            , checked = model.settingsForm.handoffEmail
            , onCheck = SetHandoffEmail
            }
        , p [ class "hint" ]
            [ text "Sends an email alert when replybots hands a WhatsApp conversation over to your team." ]
        , div [ class "email-list" ] emailRows
        , div [ class "card-actions" ]
            [ Html.button
                [ class "btn btn-secondary"
                , Html.Attributes.type_ "button"
                , onClick AddNotificationEmail
                ]
                [ text "+ Add email" ]
            , Ui.button
                { label = "Save notifications"
                , onClick = SaveNotifications
                , primary = True
                , busy = model.saving
                }
            ]
        ]


localeCard : Model -> Html Msg
localeCard model =
    let
        locales =
            [ "en-IN", "en-US" ]

        currencies =
            [ "INR", "USD" ]
    in
    Ui.colorCard "color-bar-amber"
        [ h2 [] [ text "Language and currency" ]
        , p [ class "muted" ]
            [ text "What your dashboard and invoices use." ]
        , div [ class "field" ]
            [ Html.label [] [ text "Language" ]
            , div [ class "choice-row" ]
                (List.map
                    (\tag ->
                        Html.button
                            [ classList [ ( "choice", True ), ( "is-selected", model.settingsForm.locale == tag ) ]
                            , Html.Attributes.type_ "button"
                            , onClick (SetLocaleField tag)
                            ]
                            [ text tag ]
                    )
                    locales
                )
            ]
        , div [ class "field" ]
            [ Html.label [] [ text "Currency" ]
            , div [ class "choice-row" ]
                (List.map
                    (\code ->
                        Html.button
                            [ classList [ ( "choice", True ), ( "is-selected", model.settingsForm.currency == code ) ]
                            , Html.Attributes.type_ "button"
                            , onClick (SetCurrencyField code)
                            ]
                            [ text code ]
                    )
                    currencies
                )
            ]
        , p [ class "hint" ]
            [ text "Changing currency changes what you're billed in from your next purchase. Credits you already hold keep their value." ]
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Save language"
                , onClick = SaveLocale
                , primary = True
                , busy = model.saving
                }
            ]
        ]
