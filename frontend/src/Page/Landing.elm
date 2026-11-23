module Page.Landing exposing (Model, Msg(..), init, subscriptions, update, view)

{-| The landing page and the interactive WhatsApp demo.

A visitor picks a sample small business, chats with it as a customer, and
watches replybots's built-in front-desk assistant reply briefly and hand off to
a human as soon as a real decision or quote is needed.

Three limits shape the conversation, all arriving in `/api/bootstrap`:

  - **Turn cap** (`maxUserTurns`): after this many messages the input is
    replaced by the sign-up call to action.
  - **Idle timeout** (`idleTimeoutSecs`): a conversation nobody is typing in
    swaps to the CTA. Restarts on every keystroke.
  - **Handoff** (`handed_off`): once the assistant hands off, the demo shows
    the holding-pattern pill just as a real customer thread would.

-}

import Api
import Browser.Dom as Dom
import Format
import Html exposing (Html, button, div, h1, h2, h3, input, li, p, section, span, text, ul)
import Html.Attributes exposing (attribute, class, classList, disabled, id, placeholder, value)
import Html.Events exposing (onClick, onInput, onSubmit)
import RemoteData exposing (RemoteData(..))
import Route
import Task
import Time
import Ui



-- MODEL


type alias Model =
    { chat : Maybe Chat

    -- Seconds since the visitor last typed or sent. Drives the idle timeout.
    , idleSeconds : Int
    }


type alias Chat =
    { persona : Api.DemoPersona
    , messages : List Turn
    , draft : String

    -- The in-flight reply. `Loading` renders the typing indicator.
    , pending : Api.Data Api.DemoReply
    , handoff : Bool
    }


type alias Turn =
    { role : String
    , content : String
    }


init : Model
init =
    { chat = Nothing
    , idleSeconds = 0
    }



-- UPDATE


type Msg
    = PickPersona Api.DemoPersona
    | Draft String
    | Send
    | GotReply (Api.Data Api.DemoReply)
    | CloseChat
    | Tick
    | LoginClicked
    | NoOp


update : Api.DemoConfig -> Msg -> Model -> ( Model, Cmd Msg )
update _ msg model =
    case msg of
        PickPersona persona ->
            ( { model
                | chat =
                    Just
                        { persona = persona
                        , messages = [ { role = "assistant", content = persona.opener } ]
                        , draft = ""
                        , pending = NotAsked
                        , handoff = False
                        }
                , idleSeconds = 0
              }
            , focusInput
            )

        Draft text_ ->
            ( { model
                | chat = Maybe.map (\c -> { c | draft = text_ }) model.chat
                , idleSeconds = 0
              }
            , Cmd.none
            )

        Send ->
            case model.chat of
                Just chat ->
                    let
                        body =
                            String.trim chat.draft
                    in
                    if String.isEmpty body || RemoteData.isLoading chat.pending then
                        ( model, Cmd.none )

                    else
                        let
                            priorHistory =
                                chat.messages

                            withUserTurn =
                                priorHistory ++ [ { role = "user", content = body } ]
                        in
                        ( { model
                            | chat =
                                Just
                                    { chat
                                        | messages = withUserTurn
                                        , draft = ""
                                        , pending = Loading
                                    }
                            , idleSeconds = 0
                          }
                        , Cmd.batch
                            [ Api.demoChat
                                { personaId = chat.persona.id
                                , history = priorHistory
                                , message = body
                                , handoff = chat.handoff
                                }
                                GotReply
                            , scrollToBottom
                            ]
                        )

                Nothing ->
                    ( model, Cmd.none )

        GotReply result ->
            case model.chat of
                Just chat ->
                    let
                        updated =
                            case result of
                                Success reply ->
                                    { chat
                                        | messages =
                                            chat.messages
                                                ++ [ { role = "assistant", content = reply.reply } ]
                                        , pending = NotAsked
                                        , handoff = chat.handoff || reply.handedOff
                                    }

                                _ ->
                                    { chat | pending = result }
                    in
                    ( { model | chat = Just updated }, scrollToBottom )

                Nothing ->
                    ( model, Cmd.none )

        CloseChat ->
            ( { model | chat = Nothing, idleSeconds = 0 }, Cmd.none )

        Tick ->
            ( { model | idleSeconds = model.idleSeconds + 1 }, Cmd.none )

        LoginClicked ->
            ( model, Cmd.none )

        NoOp ->
            ( model, Cmd.none )


focusInput : Cmd Msg
focusInput =
    Task.attempt (\_ -> NoOp) (Dom.focus "demo-input")


scrollToBottom : Cmd Msg
scrollToBottom =
    Dom.getViewportOf "demo-transcript"
        |> Task.andThen (\info -> Dom.setViewportOf "demo-transcript" 0 info.scene.height)
        |> Task.attempt (\_ -> NoOp)


{-| Tick once a second only while a chat is open and not already timed out.
-}
subscriptions : Api.DemoConfig -> Model -> Sub Msg
subscriptions config model =
    case model.chat of
        Just _ ->
            if model.idleSeconds < config.idleTimeoutSecs then
                Time.every 1000 (\_ -> Tick)

            else
                Sub.none

        Nothing ->
            Sub.none



-- VIEW


view : Bool -> Api.Data Api.Pricing -> Api.Data Api.DemoConfig -> Model -> Html Msg
view authBusy pricing demo model =
    div [ class "landing" ]
        [ hero authBusy pricing demo
        , case ( model.chat, demo ) of
            ( Just chat, Success config ) ->
                chatPanel authBusy config model chat

            ( _, Success config ) ->
                if config.enabled && not (List.isEmpty config.personas) then
                    personaPicker config

                else
                    text ""

            _ ->
                text ""
        , howItWorks
        ]


hero : Bool -> Api.Data Api.Pricing -> Api.Data Api.DemoConfig -> Html Msg
hero authBusy pricing demo =
    section [ class "hero" ]
        [ h1 [] [ text "Answer every WhatsApp customer, even when you're busy." ]
        , p [ class "hero-sub" ]
            [ text "Quick auto-replies for small businesses, built to hand off to you fast — greet customers, answer simple questions, and alert you the moment a person is needed." ]
        , div [ class "hero-actions" ]
            [ button
                [ Html.Attributes.type_ "button"
                , class "btn btn-primary btn-lg"
                , disabled authBusy
                , onClick LoginClicked
                ]
                [ text
                    (if authBusy then
                        "Connecting…"

                     else
                        "Continue with WhatsApp"
                    )
                ]
            , Html.a [ Route.href Route.Pricing, class "btn btn-secondary btn-lg" ]
                [ text (rateLabel pricing) ]
            ]
        , case demo of
            Success config ->
                if config.enabled && not (List.isEmpty config.personas) then
                    p [ class "hero-nudge" ] [ text "Try the live demo below — message one of these sample businesses." ]

                else
                    text ""

            _ ->
                text ""
        ]


rateLabel : Api.Data Api.Pricing -> String
rateLabel pricing =
    case pricing of
        Success p_ ->
            let
                unitPrice =
                    Api.unitPriceFor p_.defaultCurrency p_
            in
            if unitPrice > 0 then
                "From " ++ Format.milliRate "en-IN" p_.defaultCurrency unitPrice ++ " per reply"

            else
                "See pricing"

        _ ->
            "See pricing"


personaPicker : Api.DemoConfig -> Html Msg
personaPicker config =
    section [ class "demo-picker" ]
        [ Ui.sectionTitle "Pick a business to message"
        , p [ class "demo-picker-sub" ]
            [ text "replybots uses one smart built-in front-desk assistant focused on quick helpful replies and fast human handoff." ]
        , ul [ class "persona-grid" ]
            (List.map personaCard config.personas)
        ]


personaCard : Api.DemoPersona -> Html Msg
personaCard persona =
    li []
        [ button
            [ class "persona-card"
            , onClick (PickPersona persona)
            , attribute "aria-label" ("Message " ++ persona.businessName)
            ]
            [ h3 [] [ text persona.businessName ]
            , p [] [ text (persona.businessType ++ " · " ++ persona.city ++ " · " ++ persona.hours) ]
            , p [] [ text persona.summary ]
            ]
        ]


chatPanel : Bool -> Api.DemoConfig -> Model -> Chat -> Html Msg
chatPanel authBusy config model chat =
    let
        userTurns =
            chat.messages |> List.filter (\m -> m.role == "user") |> List.length

        turnsSpent =
            userTurns >= config.maxUserTurns

        idledOut =
            model.idleSeconds >= config.idleTimeoutSecs

        remainingIdle =
            max 0 (config.idleTimeoutSecs - model.idleSeconds)

        finished =
            turnsSpent || idledOut
    in
    section [ class "demo-chat" ]
        [ div [ class "demo-chat-head" ]
            [ div []
                [ h2 [] [ text chat.persona.businessName ]
                , div [ class "demo-chat-head-actions" ]
                    [ if chat.handoff then
                        span [ class "pill pill-warn" ] [ text "Handed to a human" ]

                      else
                        span [ class "pill" ] [ text "Replying automatically" ]
                    , span [ class "pill" ]
                        [ text (String.fromInt userTurns ++ " / " ++ String.fromInt config.maxUserTurns ++ " turns") ]
                    , span [ class "pill" ]
                        [ text (String.fromInt remainingIdle ++ "s idle") ]
                    ]
                ]
            , div [ class "demo-chat-head-actions" ]
                [ button [ class "btn btn-ghost", onClick CloseChat ] [ text "Start over" ]
                ]
            ]
        , div [ class "prompt-panel" ]
            [ p [ class "prompt-panel-note" ]
                [ text "replybots uses a single built-in front-desk assistant focused on quick answers and fast human handoff whenever pricing, commitments, or custom requests come up." ]
            ]
        , div [ class "demo-transcript", id "demo-transcript" ]
            (List.map bubble chat.messages
                ++ [ pendingBubble chat.pending ]
            )
        , if finished then
            closingCta authBusy idledOut

          else
            composer chat
        ]


bubble : Turn -> Html Msg
bubble turn =
    div
        [ classList
            [ ( "bubble", True )
            , ( "bubble-user", turn.role == "user" )
            , ( "bubble-assistant", turn.role /= "user" )
            ]
        ]
        [ text turn.content ]


pendingBubble : Api.Data Api.DemoReply -> Html Msg
pendingBubble pending =
    case pending of
        Loading ->
            div [ class "bubble bubble-assistant bubble-typing", attribute "aria-label" "Typing" ]
                [ span [] [], span [] [], span [] [] ]

        Failure err ->
            Ui.banner "error" (Api.errorMessage err)

        _ ->
            text ""


composer : Chat -> Html Msg
composer chat =
    Html.form [ class "demo-composer", onSubmit Send ]
        [ input
            [ id "demo-input"
            , class "demo-input"
            , placeholder "Ask them something…"
            , value chat.draft
            , onInput Draft
            , attribute "autocomplete" "off"
            , Html.Attributes.maxlength 300
            ]
            []
        , button
            [ class "btn btn-primary"
            , Html.Attributes.type_ "submit"
            , disabled (String.isEmpty (String.trim chat.draft) || RemoteData.isLoading chat.pending)
            ]
            [ text "Send" ]
        ]


closingCta : Bool -> Bool -> Html Msg
closingCta authBusy idledOut =
    div [ class "demo-cta" ]
        [ p []
            [ text
                (if idledOut then
                    "Timed out from inactivity — that's how replybots answers every WhatsApp thread day or night."

                 else
                    "That's the demo. Quick replies when customers message, and fast handoff to you when it matters."
                )
            ]
        , button
            [ Html.Attributes.type_ "button"
            , class "btn btn-primary btn-lg"
            , disabled authBusy
            , onClick LoginClicked
            ]
            [ text
                (if authBusy then
                    "Connecting…"

                 else
                    "Continue with WhatsApp"
                )
            ]
        , button [ class "btn btn-ghost", onClick CloseChat ] [ text "Try another business" ]
        ]


howItWorks : Html Msg
howItWorks =
    section [ class "how-it-works" ]
        [ Ui.sectionTitle "How it works"
        , ul [ class "steps" ]
            [ step "1. Connect WhatsApp Business via Meta"
                "Link your business number in a few clicks through Meta Embedded Signup. No new number, no app to install."
            , step "2. Choose static or AI quick-handoff replies"
                "Send an instant static message, or let replybots's built-in front-desk assistant answer briefly and hand off to your team fast."
            , step "3. Top up prepaid credits"
                "1 credit = 1 reply, static or AI. Pay as you go with Razorpay — no subscriptions or free trials."
            ]
        ]


step : String -> String -> Html Msg
step title body =
    li [ class "step" ]
        [ h3 [] [ text title ]
        , p [] [ text body ]
        ]
