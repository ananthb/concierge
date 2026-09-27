module Page.Dashboard exposing (Model, Msg, Tab(..), init, subscriptions, switchTab, update, view)

{-| The signed-in app: what's connected, how it sounds, what it costs.

Each tab owns one `RemoteData` field and fetches on first visit, so opening
the dashboard doesn't pull billing and persona data nobody asked for. Tabs are
real routes, so they're linkable and the back button works.

-}

import Api
import Format
import Html exposing (Html, a, div, h1, h2, li, p, pre, section, span, text, ul)
import Html.Attributes exposing (class, classList, href)
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

    -- Local edits, keyed by the account being edited. Only one at a time,
    -- which matches the UI: you expand one number's settings.
    , editing : Maybe ( String, Api.Reply )
    , creditsToBuy : Int
    , saving : Bool
    , notice : Maybe ( String, String )
    }


init : Tab -> ( Model, Cmd Msg )
init tab =
    let
        model =
            { tab = tab
            , channels = NotAsked
            , persona = NotAsked
            , billing = NotAsked
            , editing = Nothing
            , creditsToBuy = 0
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
            if RemoteData.isNotAsked model.persona then
                Api.getPersona GotPersona

            else
                Cmd.none

        Billing ->
            if RemoteData.isNotAsked model.billing then
                Api.getBilling GotBilling

            else
                Cmd.none

        Settings ->
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
    | SetCredits String
    | BuyCredits
    | GotOrder (Api.Data Api.Order)
    | PaymentCame Ports.PaymentOutcome
    | PaymentConfirmed (Api.Data ())


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SwitchedTo tab ->
            ( { model | tab = tab, notice = Nothing }, fetchFor tab model )

        GotChannels result ->
            ( { model | channels = result }, Cmd.none )

        GotPersona result ->
            ( { model | persona = result }, Cmd.none )

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
                settingsTab session
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
                [ Ui.card
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
                , Ui.card
                    [ h2 [] [ text "What the model is sent" ]
                    , p [ class "muted" ]
                        [ text "The first and last parts are fixed and ship with Concierge. Only the middle is yours." ]
                    , pre [ class "prompt-preview prompt-preview-fixed" ] [ text persona.preamble ]
                    , pre [ class "prompt-preview prompt-preview-middle" ] [ text persona.prompt ]
                    , pre [ class "prompt-preview prompt-preview-fixed" ] [ text persona.postamble ]
                    , p [ class "hint" ]
                        [ text "Editing the voice lives in the setup wizard for now. Changing it re-runs the safety check and pauses AI replies until it passes." ]
                    ]
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


settingsTab : Api.Session -> Html Msg
settingsTab session =
    section []
        [ Ui.card
            [ h2 [] [ text "Account" ]
            , p [] [ text session.email ]
            , p [ class "muted" ]
                [ text "Handoff emails go here — the ones telling you a conversation needs a person." ]
            ]
        , Ui.card
            [ h2 [] [ text "Close your account" ]
            , p []
                [ text "Deletes your settings, your connected number and your message metadata. Payment records are kept for tax and dispute purposes. This can't be undone." ]
            , a [ href "mailto:support@calculon.tech?subject=Delete%20my%20Concierge%20account", class "btn btn-danger" ]
                [ text "Request deletion" ]
            ]
        ]
