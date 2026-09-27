module Page.Landing exposing (Model, Msg, init, subscriptions, update, view)

{-| The landing page, and the interactive demo that is the point of it.

A visitor picks a sample business, talks to it as if they were its customer,
and watches the AI answer. "View the prompt" opens the exact middle the model
is being sent, which is the most convincing thing we can show: the product is
a prompt envelope, so letting people read it is the demo.

Three limits shape the conversation, all operator-configured and all arriving
in `/api/bootstrap`:

  - **Turn cap** (`maxUserTurns`): after this many messages the input is
    replaced by the sign-up call to action. The server enforces it too — this
    is the polite version, not the security boundary.
  - **Idle timeout** (`idleTimeoutSecs`): a conversation nobody is typing in
    swaps to the CTA. Restarts on every keystroke.
  - **Handoff**: if the model emits the handoff sentinel the server strips it
    and flags the turn. The demo then shows the holding-pattern state, which
    is exactly what a real customer would see. There is no human to escalate
    to here, so it lasts as long as the page does.

-}

import Api
import Browser.Dom as Dom
import Format
import Html exposing (Html, button, div, h1, h2, h3, input, li, p, pre, section, span, text, ul)
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
    , promptVisible : Bool
    }


type alias Chat =
    { persona : Api.DemoPersona
    , messages : List Turn
    , draft : String

    -- The in-flight reply. `Loading` renders the typing indicator.
    , pending : Api.Data Api.DemoReply

    -- Sticky once set: every later send tells the server we're in the
    -- holding pattern so it keeps using that prompt.
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
    , promptVisible = False
    }



-- UPDATE


type Msg
    = PickPersona Api.DemoPersona
    | Draft String
    | Send
    | GotReply (Api.Data Api.DemoReply)
    | TogglePrompt
    | CloseChat
    | Tick
    | NoOp


update : Api.DemoConfig -> Msg -> Model -> ( Model, Cmd Msg )
update config msg model =
    case msg of
        PickPersona persona ->
            ( { model
                | chat =
                    Just
                        { persona = persona

                        -- The greeting is the business speaking first, as it
                        -- would on WhatsApp. It's display-only: the server
                        -- strips leading assistant turns anyway, since Llama
                        -- chat templates expect the first turn to be a user's.
                        , messages = [ { role = "assistant", content = persona.greeting } ]
                        , draft = ""
                        , pending = NotAsked
                        , handoff = False
                        }
                , idleSeconds = 0
                , promptVisible = False
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
                            withUserTurn =
                                chat.messages ++ [ { role = "user", content = body } ]
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
                                { persona = chat.persona.slug
                                , messages = withUserTurn
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
                                        , handoff = reply.handoff
                                    }

                                _ ->
                                    -- Keep the failure in `pending` so the
                                    -- error renders in the transcript where
                                    -- the reply would have been.
                                    { chat | pending = result }
                    in
                    ( { model | chat = Just updated }, scrollToBottom )

                Nothing ->
                    ( model, Cmd.none )

        TogglePrompt ->
            ( { model | promptVisible = not model.promptVisible }, Cmd.none )

        CloseChat ->
            ( { model | chat = Nothing, promptVisible = False }, Cmd.none )

        Tick ->
            ( { model | idleSeconds = model.idleSeconds + 1 }, Cmd.none )

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
An idle page shouldn't hold a subscription.
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


view : Api.Pricing -> Api.DemoConfig -> Model -> Html Msg
view pricing config model =
    div [ class "landing" ]
        [ hero pricing config
        , case model.chat of
            Just chat ->
                chatPanel config model chat

            Nothing ->
                if config.enabled && not (List.isEmpty config.personas) then
                    personaPicker config

                else
                    text ""
        , howItWorks
        ]


hero : Api.Pricing -> Api.DemoConfig -> Html Msg
hero pricing config =
    section [ class "hero" ]
        [ h1 [] [ text "Answer every customer, even when you're closed." ]
        , p [ class "hero-sub" ]
            [ text "Concierge replies to your WhatsApp messages in your own voice, and hands the conversation to you the moment it matters — a price, a promise, anything it shouldn't decide alone." ]
        , div [ class "hero-actions" ]
            [ Html.a [ Html.Attributes.href "/auth/login", class "btn btn-primary btn-lg" ]
                [ text "Get started" ]
            , Html.a [ Route.href Route.Pricing, class "btn btn-secondary btn-lg" ]
                [ text ("From " ++ Format.milliRate "en-IN" pricing.currency pricing.unitPriceMilli ++ " a reply") ]
            ]
        , if config.enabled && not (List.isEmpty config.personas) then
            p [ class "hero-nudge" ] [ text "Try it below — you're the customer." ]

          else
            text ""
        ]


personaPicker : Api.DemoConfig -> Html Msg
personaPicker config =
    section [ class "demo-picker" ]
        [ Ui.sectionTitle "Pick a business to message"
        , p [ class "demo-picker-sub" ]
            [ text "These are sample businesses. You'll be the customer; Concierge answers as them." ]
        , ul [ class "persona-grid" ]
            (List.map personaCard config.personas)
        ]


personaCard : Api.DemoPersona -> Html Msg
personaCard persona =
    li []
        [ button
            [ class "persona-card"
            , onClick (PickPersona persona)
            , attribute "aria-label" ("Message " ++ persona.label)
            ]
            [ h3 [] [ text persona.label ]
            , p [] [ text persona.description ]
            ]
        ]


chatPanel : Api.DemoConfig -> Model -> Chat -> Html Msg
chatPanel config model chat =
    let
        userTurns =
            chat.messages |> List.filter (\m -> m.role == "user") |> List.length

        turnsSpent =
            userTurns >= config.maxUserTurns

        idledOut =
            model.idleSeconds >= config.idleTimeoutSecs

        finished =
            turnsSpent || idledOut
    in
    section [ class "demo-chat" ]
        [ div [ class "demo-chat-head" ]
            [ div []
                [ h2 [] [ text chat.persona.label ]
                , if chat.handoff then
                    span [ class "pill pill-warn" ] [ text "Handed to a human" ]

                  else
                    span [ class "pill" ] [ text "Replying automatically" ]
                ]
            , div [ class "demo-chat-head-actions" ]
                [ button [ class "btn btn-ghost", onClick TogglePrompt ]
                    [ text
                        (if model.promptVisible then
                            "Hide the prompt"

                         else
                            "View the prompt"
                        )
                    ]
                , button [ class "btn btn-ghost", onClick CloseChat ] [ text "Start over" ]
                ]
            ]
        , if model.promptVisible then
            promptPanel chat.persona

          else
            text ""
        , div [ class "demo-transcript", id "demo-transcript" ]
            (List.map bubble chat.messages
                ++ [ pendingBubble chat.pending ]
            )
        , if finished then
            closingCta idledOut

          else
            composer chat
        ]


promptPanel : Api.DemoPersona -> Html Msg
promptPanel persona =
    div [ class "prompt-panel" ]
        [ p [ class "prompt-panel-note" ]
            [ text "This is the middle of the prompt — the business's own voice and rules. A fixed preamble and postamble wrap it on every call and can't be edited away, which is what keeps a prompt from talking the model out of its guardrails." ]
        , pre [ class "prompt-preview" ] [ text persona.prompt ]
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

            -- The server caps this too; matching it here means the limit is
            -- visible as you type rather than arriving as a 400.
            , Html.Attributes.maxlength 300
            ]
            []
        , button
            [ class "btn btn-primary"
            , disabled (String.isEmpty (String.trim chat.draft) || RemoteData.isLoading chat.pending)
            ]
            [ text "Send" ]
        ]


closingCta : Bool -> Html Msg
closingCta idledOut =
    div [ class "demo-cta" ]
        [ p []
            [ text
                (if idledOut then
                    "Still there? That's the demo — it answers like this every time, day or night."

                 else
                    "That's the demo. It answers like this every time, day or night."
                )
            ]
        , Html.a [ Html.Attributes.href "/auth/login", class "btn btn-primary btn-lg" ]
            [ text "Set this up for my business" ]
        , button [ class "btn btn-ghost", onClick CloseChat ] [ text "Try another business" ]
        ]


howItWorks : Html Msg
howItWorks =
    section [ class "how-it-works" ]
        [ Ui.sectionTitle "How it works"
        , ul [ class "steps" ]
            [ step "Connect your WhatsApp number" "Through Meta's own signup flow. No new number, no app to install."
            , step "Describe your business once" "Pick a voice, tell it what you do and when you're open. It writes the prompt for you."
            , step "It answers; you get the hard ones" "Anything about money or a commitment is never sent on its own — Concierge holds the conversation and emails you."
            ]
        ]


step : String -> String -> Html Msg
step title body =
    li [ class "step" ]
        [ h3 [] [ text title ]
        , p [] [ text body ]
        ]
