module Page.Dashboard exposing (Model, Msg, Tab(..), init, subscriptions, switchTab, update, view)

{-| The signed-in app: what's connected, how it sounds, what it costs.

Each tab owns one `RemoteData` field and fetches on first visit, so opening
the dashboard doesn't pull billing and persona data nobody asked for. Tabs are
real routes, so they're linkable and the back button works.

-}

import Api
import Browser.Navigation as Nav
import Format
import Html exposing (Html, a, div, h1, h2, li, p, pre, section, span, text, ul)
import Html.Attributes exposing (class, classList)
import Html.Events exposing (onClick)
import Json.Encode as E
import Ports
import RemoteData exposing (RemoteData(..))
import Route
import Ui


type Tab
    = Overview
    | Persona
    | Channels
    | Billing
    | Settings


type alias Model =
    { tab : Tab
    , channels : Api.Data { accounts : List Api.WhatsAppAccount, signup : Maybe Api.Signup }
    , persona : Api.Data Api.Persona
    , billing : Api.Data Api.Billing

    -- The conversation window and the locale pair, on the Settings tab.
    , settings : Api.Data Api.Settings
    , settingsForm : SettingsForm

    -- The archetype catalog, for the voice picker. Operator-managed, so it's
    -- fetched rather than hardcoded — a fifth voice appears here without a
    -- frontend change.
    , archetypes : Api.Data (List Api.Archetype)

    -- Local edits, keyed by the account being edited. Only one at a time,
    -- which matches the UI: you expand one number's settings.
    , editing : Maybe ( String, Api.Reply )

    -- Open persona form. `Nothing` means the tab is in read-only mode.
    , personaForm : Maybe PersonaForm

    -- The composed prompt for the unsaved form, from /api/persona/preview.
    , preview : Api.Data String
    , creditsToBuy : Int

    -- Typed confirmation for closing the account. The worker requires the
    -- account's own email, so this has to match before the button works.
    , deleteConfirm : String
    , saving : Bool
    , notice : Maybe ( String, String )
    }


{-| The Settings tab's form.

The three timing knobs are held as strings because they are text inputs and
an empty box is meaningful: it means "no override, use the default". Parsing
to `Maybe Int` happens on save, so a half-typed "1" never briefly becomes a
saved value of 1.

-}
type alias SettingsForm =
    { idleGap : String
    , cooldown : String
    , history : String
    , locale : String
    , currency : String
    }


emptySettingsForm : SettingsForm
emptySettingsForm =
    { idleGap = "", cooldown = "", history = "", locale = "", currency = "" }


{-| Fill the form from what the server holds. An unset override stays an
empty box rather than showing the default, so saving an untouched form is a
no-op instead of pinning today's default.
-}
formFromSettings : Api.Settings -> SettingsForm
formFromSettings s =
    { idleGap = maybeIntToString s.conversation.idleGapMins
    , cooldown = maybeIntToString s.conversation.handoffCooldownMins
    , history = maybeIntToString s.conversation.maxHistoryMessages
    , locale = s.locale
    , currency = s.currency
    }


maybeIntToString : Maybe Int -> String
maybeIntToString =
    Maybe.map String.fromInt >> Maybe.withDefault ""


{-| Local state for the persona editor.

The chip lists are held as newline-delimited text while editing, because a
textarea is the right control for "one per line" and converting on save keeps
every keystroke from restructuring a list.

-}
type alias PersonaForm =
    { mode : String
    , builder : Api.PersonaBuilder
    , customPrompt : String
    , catchPhrasesText : String
    , offTopicsText : String
    , handoffText : String

    -- The worker's cap on a custom prompt, carried through rather than
    -- copied as a constant so the two can't drift.
    , maxLength : Int
    }


{-| Seed the form from the saved persona, so opening the editor shows what is
live rather than an empty form.
-}
formFrom : Api.Persona -> PersonaForm
formFrom persona =
    { mode = persona.mode
    , builder = persona.builder
    , customPrompt = persona.customPrompt
    , catchPhrasesText = String.join "\n" persona.builder.catchPhrases
    , offTopicsText = String.join "\n" persona.builder.offTopics
    , handoffText = String.join "\n" persona.builder.handoffConditions
    , maxLength = persona.maxCustomPrompt
    }


{-| Fold the newline-edited chip lists back into the builder for submission.
-}
formBuilder : PersonaForm -> Api.PersonaBuilder
formBuilder form =
    let
        b =
            form.builder
    in
    { b
        | catchPhrases = lines form.catchPhrasesText
        , offTopics = lines form.offTopicsText
        , handoffConditions = lines form.handoffText
    }


lines : String -> List String
lines text_ =
    text_
        |> String.lines
        |> List.map String.trim
        |> List.filter (not << String.isEmpty)


init : Tab -> ( Model, Cmd Msg )
init tab =
    let
        model =
            { tab = tab
            , channels = NotAsked
            , persona = NotAsked
            , billing = NotAsked
            , settings = NotAsked
            , settingsForm = emptySettingsForm
            , archetypes = NotAsked
            , editing = Nothing
            , personaForm = Nothing
            , preview = NotAsked
            , creditsToBuy = 0
            , deleteConfirm = ""
            , saving = False
            , notice = Nothing
            }
    in
    ( model, fetchFor tab model )


{-| Fetch the data a tab needs, unless it's already in hand. Re-visiting a tab
shouldn't re-request.
-}
fetchFor : Tab -> Model -> Cmd Msg
fetchFor tab model =
    case tab of
        Overview ->
            if RemoteData.isNotAsked model.channels then
                Api.getWhatsApp GotChannels

            else
                Cmd.none

        Channels ->
            if RemoteData.isNotAsked model.channels then
                Api.getWhatsApp GotChannels

            else
                Cmd.none

        Persona ->
            -- Both, in parallel: the editor can't render its voice picker
            -- without the catalog, and waiting for one before asking for the
            -- other would double the time to a usable form.
            Cmd.batch
                [ if RemoteData.isNotAsked model.persona then
                    Api.getPersona GotPersona

                  else
                    Cmd.none
                , if RemoteData.isNotAsked model.archetypes then
                    Api.getArchetypes GotArchetypes

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


{-| Move to another tab, keeping whatever data is already loaded.

Exposed as a function rather than by opening up `Msg`, so `Main` can drive a
tab change on navigation without the constructor set becoming part of this
module's interface.

-}
switchTab : Tab -> Model -> ( Model, Cmd Msg )
switchTab tab model =
    update (SwitchedTo tab) model



-- UPDATE


type Msg
    = SwitchedTo Tab
    | GotChannels (Api.Data { accounts : List Api.WhatsAppAccount, signup : Maybe Api.Signup })
    | GotPersona (Api.Data Api.Persona)
    | GotBilling (Api.Data Api.Billing)
    | EditReply String Api.Reply
    | CancelEdit
    | SaveReply
    | SavedReply (Api.Data Api.WhatsAppAccount)
    | GotArchetypes (Api.Data (List Api.Archetype))
    | OpenPersonaEditor
    | ClosePersonaEditor
    | SetPersonaMode String
    | EditBuilder (Api.PersonaBuilder -> Api.PersonaBuilder)
    | SetCatchPhrases String
    | SetOffTopics String
    | SetHandoff String
    | SetCustomPrompt String
    | RequestPreview
    | GotPreview (Api.Data String)
    | SavePersona
    | SavedPersona (Api.Data Api.Persona)
    | SetCredits String
    | BuyCredits
    | GotOrder (Api.Data Api.Order)
    | PaymentCame Ports.PaymentOutcome
    | PaymentConfirmed (Api.Data ())
    | GotSettings (Api.Data Api.Settings)
    | SetTiming (SettingsForm -> SettingsForm)
    | SaveTiming
    | SetLocaleField String
    | SetCurrencyField String
    | SaveLocale
    | SavedSettings (Api.Data Api.Settings)
    | SetDeleteConfirm String
    | CloseAccount
    | AccountClosed (Api.Data ())


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SwitchedTo tab ->
            ( { model | tab = tab, notice = Nothing }, fetchFor tab model )

        GotChannels result ->
            ( { model | channels = result }, Cmd.none )

        GotPersona result ->
            ( { model | persona = result }, Cmd.none )

        GotArchetypes result ->
            ( { model | archetypes = result }, Cmd.none )

        OpenPersonaEditor ->
            case model.persona of
                Success persona ->
                    ( { model
                        | personaForm = Just (formFrom persona)

                        -- The saved prompt is already on screen; seeding the
                        -- preview with it means the panel doesn't go blank
                        -- before the first Preview press.
                        , preview = Success persona.prompt
                        , notice = Nothing
                      }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        ClosePersonaEditor ->
            ( { model | personaForm = Nothing, preview = NotAsked, notice = Nothing }, Cmd.none )

        SetPersonaMode mode ->
            ( { model | personaForm = Maybe.map (\f -> { f | mode = mode }) model.personaForm }
            , Cmd.none
            )

        EditBuilder change ->
            ( { model
                | personaForm =
                    Maybe.map (\f -> { f | builder = change f.builder }) model.personaForm
              }
            , Cmd.none
            )

        SetCatchPhrases value ->
            ( { model | personaForm = Maybe.map (\f -> { f | catchPhrasesText = value }) model.personaForm }
            , Cmd.none
            )

        SetOffTopics value ->
            ( { model | personaForm = Maybe.map (\f -> { f | offTopicsText = value }) model.personaForm }
            , Cmd.none
            )

        SetHandoff value ->
            ( { model | personaForm = Maybe.map (\f -> { f | handoffText = value }) model.personaForm }
            , Cmd.none
            )

        SetCustomPrompt value ->
            ( { model | personaForm = Maybe.map (\f -> { f | customPrompt = value }) model.personaForm }
            , Cmd.none
            )

        RequestPreview ->
            -- On demand rather than on every keystroke. Each press is a
            -- request, and a debounce that fired mid-sentence would spend
            -- them on half-typed prompts.
            case model.personaForm of
                Just form ->
                    ( { model | preview = Loading }
                    , Api.previewPersona (formBuilder form) GotPreview
                    )

                Nothing ->
                    ( model, Cmd.none )

        GotPreview result ->
            ( { model | preview = result }, Cmd.none )

        SavePersona ->
            case model.personaForm of
                Just form ->
                    if form.mode == "custom" && String.isEmpty (String.trim form.customPrompt) then
                        ( { model
                            | notice =
                                Just ( "error", "Write a prompt, or switch to the guided builder." )
                          }
                        , Cmd.none
                        )

                    else
                        ( { model | saving = True, notice = Nothing }
                        , Api.savePersona
                            { mode = form.mode
                            , builder = formBuilder form
                            , customPrompt = form.customPrompt
                            }
                            SavedPersona
                        )

                Nothing ->
                    ( model, Cmd.none )

        SavedPersona result ->
            case result of
                Success persona ->
                    ( { model
                        | saving = False
                        , persona = Success persona
                        , personaForm = Nothing
                        , preview = NotAsked
                        , notice =
                            Just
                                ( "success"
                                , if persona.safetyStatus == "approved" then
                                    "Saved."

                                  else
                                    -- Worth saying plainly: the tenant has
                                    -- just turned their own AI replies off
                                    -- until the classifier comes back.
                                    "Saved. AI replies are paused while we check the new prompt."
                                )
                      }
                    , Cmd.none
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        GotBilling result ->
            let
                -- Seed the slider at the minimum the operator allows, so the
                -- buy button is valid the moment the page renders.
                credits =
                    case result of
                        Success billing ->
                            max model.creditsToBuy billing.minCredits

                        _ ->
                            model.creditsToBuy
            in
            ( { model | billing = result, creditsToBuy = credits }, Cmd.none )

        EditReply id reply ->
            ( { model | editing = Just ( id, reply ), notice = Nothing }, Cmd.none )

        CancelEdit ->
            ( { model | editing = Nothing }, Cmd.none )

        SaveReply ->
            case ( model.editing, model.channels ) of
                ( Just ( id, reply ), Success { accounts } ) ->
                    let
                        name =
                            accounts
                                |> List.filter (\a -> a.id == id)
                                |> List.head
                                |> Maybe.map .name
                                |> Maybe.withDefault ""
                    in
                    ( { model | saving = True }
                    , Api.saveWhatsApp id { name = name, reply = reply } SavedReply
                    )

                _ ->
                    ( model, Cmd.none )

        SavedReply result ->
            case result of
                Success account ->
                    ( { model
                        | saving = False
                        , editing = Nothing
                        , notice = Just ( "success", "Saved." )

                        -- Patch the list in place rather than re-fetching.
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
                    | notice =
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
                ( { model | saving = True }
                , Api.verifyPayment
                    { orderId = outcome.orderId
                    , paymentId = outcome.paymentId
                    , signature = outcome.signature
                    }
                    PaymentConfirmed
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

        SetTiming change ->
            ( { model | settingsForm = change model.settingsForm }, Cmd.none )

        SaveTiming ->
            case model.settings of
                Success settings ->
                    let
                        form =
                            model.settingsForm

                        -- An empty box clears the override; anything that
                        -- isn't a number keeps what the server already has,
                        -- so a typo can't silently reset a knob to default.
                        read text stored =
                            if String.trim text == "" then
                                Nothing

                            else
                                case String.toInt (String.trim text) of
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
                            , handoffCooldownMins = read form.cooldown conversation.handoffCooldownMins
                            , maxHistoryMessages = read form.history conversation.maxHistoryMessages
                        }
                        SavedSettings
                    )

                _ ->
                    ( model, Cmd.none )

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
                    -- A real page load, not a route change: the session
                    -- cookie is gone and every cached payload in this model
                    -- belongs to an account that no longer exists.
                    ( { model | saving = False }, Nav.load "/" )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        PaymentConfirmed result ->
            case result of
                Success () ->
                    -- Credits arrive via the webhook, which may land after
                    -- this. Re-fetch so the balance is as fresh as it can be,
                    -- and say so rather than claiming the credits are there.
                    ( { model
                        | saving = False
                        , billing = Loading
                        , notice =
                            Just
                                ( "success"
                                , "Payment received. Your balance updates within a few seconds."
                                )
                      }
                    , Api.getBilling GotBilling
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )


subscriptions : Sub Msg
subscriptions =
    Ports.paymentOutcome PaymentCame



-- VIEW


view : Api.Session -> Model -> Html Msg
view session model =
    div [ class "dashboard" ]
        [ h1 [] [ text (greeting session) ]
        , if not session.aiReady then
            Ui.banner "info"
                "AI replies are paused while we check your prompt. Canned replies still go out."

          else
            text ""
        , tabs model.tab
        , case model.notice of
            Just ( kind, message ) ->
                Ui.banner kind message

            Nothing ->
                text ""
        , case model.tab of
            Overview ->
                overviewTab model

            Channels ->
                channelsTab model

            Persona ->
                personaTab model

            Billing ->
                billingTab model

            Settings ->
                settingsTab session model
        ]


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
            [ ( Overview, Route.Dashboard, "Overview" )
            , ( Channels, Route.DashboardChannels, "WhatsApp" )
            , ( Persona, Route.DashboardPersona, "Voice" )
            , ( Billing, Route.DashboardBilling, "Credits" )
            , ( Settings, Route.DashboardSettings, "Settings" )
            ]
        )


overviewTab : Model -> Html Msg
overviewTab model =
    Ui.remote model.channels <|
        \{ accounts } ->
            section []
                [ if List.isEmpty accounts then
                    Ui.card
                        [ h2 [] [ text "No number connected" ]
                        , p [] [ text "Concierge can't answer anything until a WhatsApp number is attached." ]
                        , Ui.linkButton
                            { label = "Connect WhatsApp"
                            , url = Route.toPath Route.DashboardChannels
                            , primary = True
                            }
                        ]

                  else
                    Ui.card
                        [ h2 [] [ text "Answering on" ]
                        , ul [ class "connected-list" ]
                            (List.map
                                (\a ->
                                    li [ class "connected-item" ]
                                        [ span [ class "connected-name" ] [ text a.name ]
                                        , span [ class "connected-phone" ] [ text a.phoneNumber ]
                                        , span
                                            [ classList
                                                [ ( "pill", True )
                                                , ( "pill-ok", a.reply.enabled )
                                                , ( "pill-warn", not a.reply.enabled )
                                                ]
                                            ]
                                            [ text
                                                (if a.reply.enabled then
                                                    "Auto-reply on"

                                                 else
                                                    "Auto-reply off"
                                                )
                                            ]
                                        ]
                                )
                                accounts
                            )
                        ]
                ]


channelsTab : Model -> Html Msg
channelsTab model =
    Ui.remote model.channels <|
        \{ accounts, signup } ->
            section []
                (if List.isEmpty accounts then
                    [ Ui.card
                        [ h2 [] [ text "Connect your WhatsApp number" ]
                        , case signup of
                            Just _ ->
                                Ui.linkButton
                                    { label = "Connect with Meta"
                                    , url = "/wizard"
                                    , primary = True
                                    }

                            Nothing ->
                                Ui.banner "info" "WhatsApp signup isn't configured on this deployment."
                        ]
                    ]

                 else
                    List.map (accountCard model) accounts
                )


accountCard : Model -> Api.WhatsAppAccount -> Html Msg
accountCard model account =
    let
        editing =
            case model.editing of
                Just ( id, reply ) ->
                    if id == account.id then
                        Just reply

                    else
                        Nothing

                Nothing ->
                    Nothing
    in
    Ui.card
        [ h2 [] [ text account.name ]
        , p [ class "muted" ] [ text account.phoneNumber ]
        , case editing of
            Just reply ->
                div []
                    [ Ui.toggle
                        { id = "reply-enabled"
                        , label = "Reply automatically to new messages"
                        , checked = reply.enabled
                        , onCheck = \v -> EditReply account.id { reply | enabled = v }
                        }
                    , modePicker account.id reply
                    , Ui.textarea
                        { id = "reply-text"
                        , label =
                            if reply.mode == "prompt" then
                                "What should the AI do?"

                            else
                                "Message to send"
                        , value = reply.text
                        , hint =
                            if reply.mode == "prompt" then
                                "This sits after your voice prompt. One credit per reply."

                            else
                                "Sent word for word. Costs nothing."
                        , rows = 4
                        , onInput = \v -> EditReply account.id { reply | text = v }
                        }
                    , Ui.field
                        { id = "wait"
                        , label = "Wait before replying (seconds)"
                        , value = String.fromInt reply.waitSeconds
                        , hint = "Gives customers a moment to finish typing, so a burst of messages gets one answer. 0 replies instantly."
                        , required = False
                        , onInput =
                            \v ->
                                EditReply account.id
                                    { reply | waitSeconds = String.toInt v |> Maybe.withDefault reply.waitSeconds }
                        }
                    , div [ class "card-actions" ]
                        [ Ui.button { label = "Cancel", onClick = CancelEdit, primary = False, busy = False }
                        , Ui.button { label = "Save", onClick = SaveReply, primary = True, busy = model.saving }
                        ]
                    ]

            Nothing ->
                div []
                    [ p []
                        [ text
                            (if account.reply.enabled then
                                if account.reply.mode == "prompt" then
                                    "Answering with AI."

                                else
                                    "Sending a fixed reply."

                             else
                                "Not replying."
                            )
                        ]
                    , Ui.button
                        { label = "Edit replies"
                        , onClick = EditReply account.id account.reply
                        , primary = False
                        , busy = False
                        }
                    ]
        ]


modePicker : String -> Api.Reply -> Html Msg
modePicker id reply =
    div [ class "field" ]
        [ Html.label [] [ text "Reply with" ]
        , div [ class "choice-row" ]
            (List.map
                (\( wire, label_ ) ->
                    Html.button
                        [ classList [ ( "choice", True ), ( "is-selected", reply.mode == wire ) ]
                        , Html.Attributes.type_ "button"
                        , onClick (EditReply id { reply | mode = wire })
                        ]
                        [ text label_ ]
                )
                [ ( "prompt", "AI" ), ( "canned", "Fixed message" ) ]
            )
        ]


personaTab : Model -> Html Msg
personaTab model =
    Ui.remote model.persona <|
        \persona ->
            section []
                [ safetyCard persona
                , case model.personaForm of
                    Just form ->
                        personaEditor model form

                    Nothing ->
                        promptCard persona
                ]


safetyCard : Api.Persona -> Html Msg
safetyCard persona =
    Ui.card
        [ h2 [] [ text "Safety check" ]
        , case persona.safetyStatus of
            "approved" ->
                Ui.banner "success" "Approved. AI replies are going out."

            "pending" ->
                Ui.banner "info" "We're checking this prompt. AI replies are paused until it clears."

            _ ->
                Ui.banner "error"
                    (Maybe.withDefault
                        "This prompt didn't pass our safety check."
                        persona.safetyReason
                    )
        ]


{-| Read-only view of what is live, with the fixed bookends shown around it.
-}
promptCard : Api.Persona -> Html Msg
promptCard persona =
    Ui.card
        [ h2 [] [ text "What the model is sent" ]
        , p [ class "muted" ]
            [ text "The first and last parts are fixed and ship with Concierge. Only the middle is yours." ]
        , pre [ class "prompt-preview prompt-preview-fixed" ] [ text persona.preamble ]
        , pre [ class "prompt-preview prompt-preview-middle" ] [ text persona.prompt ]
        , pre [ class "prompt-preview prompt-preview-fixed" ] [ text persona.postamble ]
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Edit your voice"
                , onClick = OpenPersonaEditor
                , primary = True
                , busy = False
                }
            ]
        ]


{-| The editor. Two modes, and only ever one: a persona is a guided builder or
a raw prompt, never a blend, so there is exactly one source for the active
prompt.
-}
personaEditor : Model -> PersonaForm -> Html Msg
personaEditor model form =
    Ui.card
        [ h2 [] [ text "Edit your voice" ]
        , Ui.banner "info"
            "Saving re-runs the safety check. AI replies pause until the new prompt is approved; fixed replies keep going out."
        , modeSwitch form.mode
        , if form.mode == "custom" then
            customFields model form

          else
            builderFields model form
        , previewPanel model form
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Cancel"
                , onClick = ClosePersonaEditor
                , primary = False
                , busy = False
                }
            , Ui.button
                { label = "Save"
                , onClick = SavePersona
                , primary = True
                , busy = model.saving
                }
            ]
        ]


modeSwitch : String -> Html Msg
modeSwitch current =
    div [ class "field" ]
        [ Html.label [] [ text "How you want to write it" ]
        , div [ class "choice-row" ]
            (List.map
                (\( wire, label_ ) ->
                    Html.button
                        [ classList [ ( "choice", True ), ( "is-selected", current == wire ) ]
                        , Html.Attributes.type_ "button"
                        , onClick (SetPersonaMode wire)
                        ]
                        [ text label_ ]
                )
                [ ( "builder", "Guided" ), ( "custom", "Write the prompt myself" ) ]
            )
        ]


builderFields : Model -> PersonaForm -> Html Msg
builderFields model form =
    let
        b =
            form.builder
    in
    div []
        [ voicePicker model b.archetypeSlug
        , Ui.field
            { id = "biz_name"
            , label = "Business name"
            , value = b.bizName
            , hint = "How the AI refers to you."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | bizName = v })
            }
        , Ui.field
            { id = "biz_type"
            , label = "What you do"
            , value = b.bizType
            , hint = "A few words: florist, dental clinic, tuition centre."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | bizType = v })
            }
        , Ui.field
            { id = "city"
            , label = "Where you are"
            , value = b.city
            , hint = ""
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | city = v })
            }
        , Ui.field
            { id = "hours"
            , label = "When you're open"
            , value = b.hours
            , hint = "So it can say when you'll be back instead of guessing."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | hours = v })
            }
        , Ui.field
            { id = "goal"
            , label = "What each conversation should aim for"
            , value = b.goal
            , hint = "For example: get them to book a slot."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | goal = v })
            }
        , Ui.field
            { id = "goal_url"
            , label = "A link worth sending"
            , value = b.goalUrl
            , hint = "Your booking page, menu or catalogue. Anything that isn't a plain http(s) link is dropped."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | goalUrl = v })
            }
        , Ui.textarea
            { id = "catch_phrases"
            , label = "Things you'd actually say"
            , value = form.catchPhrasesText
            , hint = "One per line, up to five. These make it sound like you rather than like an assistant."
            , rows = 3
            , onInput = SetCatchPhrases
            }
        , Ui.textarea
            { id = "off_topics"
            , label = "Stay off these"
            , value = form.offTopicsText
            , hint = "One per line. A draft that strays into one of these is withheld and you're emailed instead."
            , rows = 3
            , onInput = SetOffTopics
            }
        , Ui.field
            { id = "never"
            , label = "Never say"
            , value = b.never
            , hint = "One thing it must not promise. Also checked against every draft before it's sent."
            , required = False
            , onInput = \v -> EditBuilder (\x -> { x | never = v })
            }
        , Ui.textarea
            { id = "handoff"
            , label = "When should it stop and fetch you?"
            , value = form.handoffText
            , hint = "One per line. Prices and commitments already stop it automatically — these are your own additions."
            , rows = 3
            , onInput = SetHandoff
            }
        ]


voicePicker : Model -> String -> Html Msg
voicePicker model current =
    div [ class "field" ]
        [ Html.label [] [ text "Voice" ]
        , Ui.remote model.archetypes <|
            \archetypes ->
                if List.isEmpty archetypes then
                    Ui.banner "info" "No voices are available right now. Try again shortly."

                else
                    ul [ class "voice-grid" ]
                        (List.map
                            (\archetype ->
                                li []
                                    [ Html.button
                                        [ classList
                                            [ ( "voice-card", True )
                                            , ( "is-selected", current == archetype.slug )
                                            ]
                                        , Html.Attributes.type_ "button"
                                        , onClick
                                            (EditBuilder
                                                (\x -> { x | archetypeSlug = archetype.slug })
                                            )
                                        ]
                                        [ h2 [] [ text archetype.label ]
                                        , p [] [ text archetype.description ]
                                        ]
                                    ]
                            )
                            archetypes
                        )
        ]


customFields : Model -> PersonaForm -> Html Msg
customFields model form =
    let
        used =
            String.length form.customPrompt
    in
    div []
        [ Ui.textarea
            { id = "custom_prompt"
            , label = "Your prompt"
            , value = form.customPrompt
            , hint =
                "This replaces the guided fields entirely. The fixed preamble and postamble still wrap it — they can't be edited away."
            , rows = 14
            , onInput = SetCustomPrompt
            }
        , p
            [ classList
                [ ( "hint", True )
                , ( "hint-warn", used > form.maxLength )
                ]
            ]
            [ text
                (String.fromInt used
                    ++ " / "
                    ++ String.fromInt form.maxLength
                    ++ " characters"
                    ++ (if used > form.maxLength then
                            " — anything past the limit is dropped on save."

                        else
                            ""
                       )
                )
            ]
        ]


{-| The composed middle for the _unsaved_ form.

Worth its own panel: the guided fields don't obviously map onto a prompt, and
the point of the product is that you can read exactly what the model is told.

-}
previewPanel : Model -> PersonaForm -> Html Msg
previewPanel model form =
    div [ class "preview-panel" ]
        [ div [ class "preview-panel-head" ]
            [ h2 [] [ text "What this composes to" ]
            , Ui.button
                { label = "Refresh preview"
                , onClick = RequestPreview
                , primary = False
                , busy = RemoteData.isLoading model.preview
                }
            ]
        , if form.mode == "custom" then
            pre [ class "prompt-preview prompt-preview-middle" ]
                [ text
                    (if String.isEmpty (String.trim form.customPrompt) then
                        "Nothing yet."

                     else
                        form.customPrompt
                    )
                ]

          else
            Ui.remote model.preview <|
                \prompt -> pre [ class "prompt-preview prompt-preview-middle" ] [ text prompt ]
        ]


billingTab : Model -> Html Msg
billingTab model =
    Ui.remote model.billing <|
        \billing ->
            section []
                [ Ui.card
                    [ h2 [] [ text "Balance" ]
                    , p [ class "balance" ]
                        [ text (Format.count "en-IN" billing.balance ++ " replies") ]
                    , p [ class "muted" ]
                        [ text (Format.count "en-IN" billing.repliesUsed ++ " used so far.") ]
                    ]
                , if billing.metered then
                    Ui.card
                        [ h2 [] [ text "Buy replies" ]
                        , p []
                            [ text
                                (Format.milliRate "en-IN" billing.currency billing.unitPriceMilli
                                    ++ " each. No packs, no tiers — buy any number."
                                )
                            ]
                        , Html.input
                            [ Html.Attributes.type_ "range"
                            , Html.Attributes.min (String.fromInt billing.minCredits)
                            , Html.Attributes.max (String.fromInt billing.maxCredits)
                            , Html.Attributes.step (String.fromInt billing.minCredits)
                            , Html.Attributes.value (String.fromInt model.creditsToBuy)
                            , Html.Events.onInput SetCredits
                            , class "credit-slider"
                            ]
                            []
                        , p [ class "credit-total" ]
                            [ text
                                (Format.count "en-IN" model.creditsToBuy
                                    ++ " replies for "
                                    ++ Format.money "en-IN"
                                        billing.currency
                                        (totalFor model.creditsToBuy billing.unitPriceMilli)
                                )
                            ]
                        , Ui.button
                            { label = "Pay"
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
                , if List.isEmpty billing.credits then
                    text ""

                  else
                    Ui.card
                        [ h2 [] [ text "Where your balance came from" ]
                        , ul [ class "ledger" ]
                            (List.map
                                (\credit ->
                                    li []
                                        [ span [] [ text (Format.count "en-IN" credit.amount) ]
                                        , span [ class "muted" ] [ text credit.source ]
                                        , case credit.expiresAt of
                                            Just when ->
                                                span [ class "muted" ] [ text ("expires " ++ String.left 10 when) ]

                                            Nothing ->
                                                span [ class "muted" ] [ text "never expires" ]
                                        ]
                                )
                                billing.credits
                            )
                        ]
                ]


{-| Mirror of `billing::calculate_total` in the worker: credits × milli price,
rounded to the nearest minor unit. Duplicated so the slider can show a total
without a request per drag; the worker's figure is authoritative on checkout.
-}
totalFor : Int -> Int -> Int
totalFor credits milliPrice =
    (credits * milliPrice + 500) // 1000


{-| The conversation window.

Three knobs the pipeline has always honoured and nothing could set. Each box
is empty when the account has no override, with the default shown as the
placeholder — so the form reads "leave it alone unless you mean it", and
emptying a box puts that knob back on the default.

-}
timingCard : Model -> Api.Settings -> Html Msg
timingCard model settings =
    let
        c =
            settings.conversation

        form =
            model.settingsForm

        knob id label hint value bound change =
            Ui.field
                { id = id
                , label = label
                , value = value
                , hint =
                    hint
                        ++ " Default "
                        ++ String.fromInt bound
                        ++ "; leave empty to use it."
                , required = False
                , onInput = \v -> SetTiming (change v)
                }
    in
    Ui.card
        [ h2 [] [ text "Conversation timing" ]
        , p [ class "muted" ]
            [ text "How Concierge decides where one conversation ends and the next begins." ]
        , knob "idle_gap_mins"
            "Minutes of silence before a new conversation"
            "After this long without a message, the next one starts fresh — no earlier context."
            form.idleGap
            c.defaults.idleGapMins
            (\v f -> { f | idleGap = v })
        , knob "handoff_cooldown_mins"
            "Minutes to keep holding after a handoff"
            "Once a conversation is handed to you, Concierge answers in the holding voice for this long, then goes quiet."
            form.cooldown
            c.defaults.handoffCooldownMins
            (\v f -> { f | cooldown = v })
        , knob "max_history_messages"
            "Recent messages sent to the AI"
            "More context costs more per reply and can drown the instructions."
            form.history
            c.defaults.maxHistoryMessages
            (\v f -> { f | history = v })
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Save timing"
                , onClick = SaveTiming
                , primary = True
                , busy = model.saving
                }
            ]
        ]


localeCard : Model -> Api.Settings -> Html Msg
localeCard model settings =
    Ui.card
        [ h2 [] [ text "Language and currency" ]
        , p [ class "muted" ]
            [ text "What your dashboard and invoices are written in. Your customers always get replies in the voice you set, whatever this says." ]
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
                    settings.locales
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
                    settings.currencies
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


settingsTab : Api.Session -> Model -> Html Msg
settingsTab session model =
    let
        confirmed =
            String.toLower (String.trim model.deleteConfirm) == String.toLower session.email
    in
    section []
        [ Ui.remote model.settings
            (\settings ->
                div []
                    [ timingCard model settings
                    , localeCard model settings
                    ]
            )
        , Ui.card
            [ h2 [] [ text "Account" ]
            , p [] [ text session.email ]
            , p [ class "muted" ]
                [ text "Handoff emails go here — the ones telling you a conversation needs a person." ]
            ]
        , Ui.card
            [ h2 [] [ text "Close your account" ]
            , p []
                [ text "Deletes your settings, your connected number, your persona and your message metadata. Payment records are kept with your account identifier removed, because tax and dispute rules require it. This cannot be undone." ]
            , Ui.field
                { id = "confirm_email"
                , label = "Type " ++ session.email ++ " to confirm"
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
