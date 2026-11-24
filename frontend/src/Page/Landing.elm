module Page.Landing exposing (Model, Msg(..), demoPage, init, subscriptions, update, view)

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
import Dict
import Format
import Html exposing (Html, button, div, h1, h2, h3, input, li, p, section, span, text, ul)
import Html.Attributes exposing (attribute, class, classList, disabled, id, placeholder, value)
import Html.Events exposing (onClick, onInput, onSubmit)
import Html.Keyed as Keyed
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
    , reducedMotion : Bool
    , wordIndex : Int
    , phase : TypePhase
    }


type TypePhase
    = Holding Int
    | Erasing Int
    | PausingEmpty Int
    | Typing Int


heroWords : List String
heroWords =
    [ "small businesses"
    , "creators"
    , "design studios"
    , "artists"
    , "side-hustles"
    , "event venues"
    , "local shops"
    ]


wordAt : Int -> String
wordAt idx =
    let
        len =
            List.length heroWords
    in
    heroWords
        |> List.drop (modBy len idx)
        |> List.head
        |> Maybe.withDefault "small businesses"


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


init : Bool -> Model
init reducedMotion =
    { chat = Nothing
    , idleSeconds = 0
    , reducedMotion = reducedMotion
    , wordIndex = 0
    , phase = Holding 34
    }



-- UPDATE


type Msg
    = PickPersona Api.DemoPersona
    | Draft String
    | Send
    | GotReply (Api.Data Api.DemoReply)
    | CloseChat
    | Tick
    | TypeTick
    | ReducedSwapTick
    | LoginClicked
    | NoOp


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        PickPersona persona ->
            case model.chat of
                Just current ->
                    if current.persona.id == persona.id then
                        ( { model | chat = Nothing, idleSeconds = 0 }, Cmd.none )

                    else
                        openPersona persona model

                Nothing ->
                    openPersona persona model

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

        TypeTick ->
            ( stepTypewriter model, Cmd.none )

        ReducedSwapTick ->
            ( { model | wordIndex = modBy (List.length heroWords) (model.wordIndex + 1) }, Cmd.none )

        LoginClicked ->
            ( model, Cmd.none )

        NoOp ->
            ( model, Cmd.none )


stepTypewriter : Model -> Model
stepTypewriter model =
    let
        currentWord =
            wordAt model.wordIndex
    in
    case model.phase of
        Holding n ->
            if n > 1 then
                { model | phase = Holding (n - 1) }

            else
                { model | phase = Erasing (max 0 (String.length currentWord - 1)) }

        Erasing len ->
            if len > 0 then
                { model | phase = Erasing (len - 1) }

            else
                { model | phase = PausingEmpty 4 }

        PausingEmpty n ->
            if n > 1 then
                { model | phase = PausingEmpty (n - 1) }

            else
                let
                    nextIdx =
                        modBy (List.length heroWords) (model.wordIndex + 1)
                in
                { model | wordIndex = nextIdx, phase = Typing 1 }

        Typing len ->
            if len < String.length currentWord then
                { model | phase = Typing (len + 1) }

            else
                { model | phase = Holding 34 }


openPersona : Api.DemoPersona -> Model -> ( Model, Cmd Msg )
openPersona persona model =
    ( { model
        | chat =
            Just
                { persona = persona
                , messages = []
                , draft = ""
                , pending = NotAsked
                , handoff = False
                }
        , idleSeconds = 0
      }
    , focusInput
    )


focusInput : Cmd Msg
focusInput =
    Task.attempt (\_ -> NoOp) (Dom.focus "demo-input")


scrollToBottom : Cmd Msg
scrollToBottom =
    Dom.getViewportOf "demo-transcript"
        |> Task.andThen (\info -> Dom.setViewportOf "demo-transcript" 0 info.scene.height)
        |> Task.attempt (\_ -> NoOp)


{-| Tick the hero word cycler and, when a demo chat is open, the idle timer.
-}
subscriptions : Maybe Api.DemoConfig -> Model -> Sub Msg
subscriptions maybeConfig model =
    let
        idleSub =
            case ( maybeConfig, model.chat ) of
                ( Just config, Just _ ) ->
                    if model.idleSeconds < config.idleTimeoutSecs then
                        Time.every 1000 (\_ -> Tick)

                    else
                        Sub.none

                _ ->
                    Sub.none

        heroSub =
            if model.reducedMotion then
                Time.every 3500 (\_ -> ReducedSwapTick)

            else
                Time.every 65 (\_ -> TypeTick)
    in
    Sub.batch [ idleSub, heroSub ]



-- VIEW


view : Bool -> Api.Data Api.Pricing -> Api.Data Api.DemoConfig -> Model -> Html Msg
view authBusy pricing demo model =
    div [ class "landing" ]
        [ hero authBusy demo model
        , howItWorks
        , featuresSection
        , pricingSection pricing
        ]


hero : Bool -> Api.Data Api.DemoConfig -> Model -> Html Msg
hero authBusy demo model =
    let
        demoEnabled =
            case demo of
                Success config ->
                    config.enabled && not (List.isEmpty config.personas)

                _ ->
                    True

        currentWord =
            wordAt model.wordIndex

        displayedWord =
            if model.reducedMotion then
                currentWord

            else
                case model.phase of
                    Holding _ ->
                        currentWord

                    Erasing len ->
                        String.left len currentWord

                    PausingEmpty _ ->
                        ""

                    Typing len ->
                        String.left len currentWord
    in
    section [ class "hero" ]
        [ h1 []
            (text "WhatsApp auto-replies for "
                :: (if model.reducedMotion then
                        [ Keyed.node "span"
                            [ class "hero-word" ]
                            [ ( String.fromInt model.wordIndex
                              , span [ class "hero-word-fade" ] [ text displayedWord ]
                              )
                            ]
                        ]

                    else
                        [ span [ class "hero-word" ] [ text displayedWord ]
                        , span [ class "cursor-blink", attribute "aria-hidden" "true" ] []
                        ]
                   )
            )
        , p [ class "hero-sub" ]
            [ text "Static replies are free. 1 credit = 1 AI-generated reply." ]
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
            , if demoEnabled then
                Html.a [ Route.href Route.Demo, class "btn btn-secondary btn-lg" ]
                    [ text "Try the demo" ]

              else
                Html.a [ Html.Attributes.href "#pricing", class "btn btn-secondary btn-lg" ]
                    [ text "See pricing" ]
            ]
        ]


featuresSection : Html Msg
featuresSection =
    section [ id "features", class "landing-features" ]
        [ Ui.sectionTitle "Features"
        , div [ class "feature-list" ]
            [ featureCard "Static or AI quick-handoff replies"
                "Choose a fixed static message or let replybots's built-in front-desk assistant answer briefly on your WhatsApp Business number."
            , featureCard "Fast human handoff + email alert"
                "The moment a customer asks for a quote, a commitment, or personal attention, replybots hands off cleanly and optionally emails you so you can step in."
            , featureCard "Burst message batching"
                "Wait 0 to 30 seconds for customers who send multiple short messages in a row, so they get one coherent reply instead of three."
            , featureCard "Conversation session window"
                "Configure the idle gap, handoff cooldown, and recent message context so threads stay relevant without repeating stale history."
            , featureCard "AI safety & guardrails"
                "Every reply passes through AI safety and prompt-injection guardrails designed to stop off-topic drift and unauthorized promises."
            , featureCard "Metadata-only privacy"
                "Customer message text passes through in memory to produce a reply and is never stored in our database — only channel, direction, sender, recipient, and timestamp are kept."
            ]
        ]


featureCard : String -> String -> Html Msg
featureCard title body =
    div [ class "feature" ]
        [ h3 [] [ text title ]
        , p [] [ text body ]
        ]


pricingSection : Api.Data Api.Pricing -> Html Msg
pricingSection pricing =
    section [ id "pricing", class "landing-pricing" ]
        [ Ui.sectionTitle "Simple, prepaid pricing"
        , p [ class "pricing-lead" ]
            [ text "Static replies are free. 1 credit = 1 AI-generated reply. No subscriptions, no per-seat fees, no expiry." ]
        , case pricing of
            Success p_ ->
                div [ class "price-cards__inner" ]
                    (List.map landingRateCard (Dict.toList p_.unitPriceMilli))

            _ ->
                text ""
        , ul [ class "pricing-rules" ]
            [ li [] [ text "No charge if replybots holds back a message or AI generation fails." ]
            , li []
                [ text
                    (case pricing of
                        Success p_ ->
                            "Top up anytime in your dashboard ("
                                ++ Format.count "en-IN" p_.minCredits
                                ++ " to "
                                ++ Format.count "en-IN" p_.maxCredits
                                ++ " credits per order)."

                        _ ->
                            "Top up anytime in your dashboard."
                    )
                ]
            , li [] [ text "Purchased credits never expire." ]
            ]
        ]


landingRateCard : ( String, Int ) -> Html Msg
landingRateCard ( code, unitPriceMilli ) =
    div [ class "price-card" ]
        [ h3 [] [ text code ]
        , p [ class "price" ]
            [ text (Format.milliRate "en-IN" code unitPriceMilli) ]
        , p [ class "muted" ] [ text "per reply" ]
        ]


demoPage : Bool -> Api.Data Api.DemoConfig -> Model -> Html Msg
demoPage authBusy demo model =
    div [ class "page page-demo" ]
        [ h1 [] [ text "Live demo" ]
        , p [ class "demo-lead" ]
            [ text "Pick a bot persona below to test a simulated WhatsApp chat." ]
        , case demo of
            Success config ->
                if config.enabled && not (List.isEmpty config.personas) then
                    personaPicker authBusy config model

                else
                    p [ class "muted" ] [ text "The live demo is currently unavailable." ]

            Loading ->
                Ui.spinner

            _ ->
                text ""
        ]


personaPicker : Bool -> Api.DemoConfig -> Model -> Html Msg
personaPicker authBusy config model =
    let
        selectedId =
            Maybe.map (.persona >> .id) model.chat

        selectedWithIndex =
            model.chat
                |> Maybe.andThen
                    (\chat ->
                        config.personas
                            |> List.indexedMap Tuple.pair
                            |> List.filter (\( _, p ) -> p.id == chat.persona.id)
                            |> List.head
                            |> Maybe.map (\( idx, _ ) -> ( idx, chat ))
                    )
    in
    div [ class "persona-demo-wrap" ]
        (List.indexedMap (personaBtn selectedId) config.personas
            ++ [ case selectedWithIndex of
                    Just ( idx, chat ) ->
                        chatPanel authBusy config model idx chat

                    Nothing ->
                        text ""
               ]
        )


personaBtn : Maybe String -> Int -> Api.DemoPersona -> Html Msg
personaBtn selectedId index persona =
    let
        isSelected =
            selectedId == Just persona.id
    in
    button
        [ classList
            [ ( "persona-btn", True )
            , ( "persona-slot-" ++ String.fromInt index, True )
            , ( "is-active", isSelected )
            ]
        , Html.Attributes.type_ "button"
        , onClick (PickPersona persona)
        , attribute "aria-pressed"
            (if isSelected then
                "true"

             else
                "false"
            )
        , attribute "aria-label" ("Message " ++ persona.label ++ " — " ++ persona.businessName)
        ]
        [ div [ class "persona-head" ]
            [ h3 [ class "persona-label" ] [ text persona.label ]
            , span [ class "persona-tag" ] [ text persona.tag ]
            ]
        , p [ class "persona-desc" ] [ text persona.businessName ]
        ]


chatPanel : Bool -> Api.DemoConfig -> Model -> Int -> Chat -> Html Msg
chatPanel authBusy config model index chat =
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
    section [ class ("demo-box demo-chat demo-after-" ++ String.fromInt index) ]
        [ div [ class "demo-badge-bar" ]
            [ span [ class "demo-badge" ] [ text "DEMO — SIMULATED CHAT" ]
            ]
        , div [ class "demo-chat-head" ]
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
            [ p [ class "prompt-preview prompt-preview-middle" ]
                [ text ("User prompt: " ++ chat.persona.prompt) ]
            , p [ class "prompt-panel-note" ]
                [ text "Shared system prompt + quick human handoff built in." ]
            ]
        , div [ class "demo-transcript", id "demo-transcript" ]
            (if List.isEmpty chat.messages && not (RemoteData.isLoading chat.pending) then
                [ p [ class "demo-empty" ]
                    [ text "Send a message as a customer to test this bot." ]
                , pendingBubble chat.pending
                ]

             else
                List.map bubble chat.messages
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
            , ( "demo-msg", True )
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
    section [ id "how-it-works", class "how-it-works" ]
        [ Ui.sectionTitle "How it works"
        , ul [ class "steps" ]
            [ step "color-bar-blue"
                "1. Login with WhatsApp"
                "Connect your WhatsApp Business number via Meta Embedded Signup. No new number or app needed."
            , step "color-bar-green"
                "2. Pick or edit 1 bot prompt"
                "Choose a starter bot prompt or write your own (up to 600 chars), or send a static reply."
            , step "color-bar-amber"
                "3. 1 credit = 1 AI reply"
                "Static replies are free. 1 credit = 1 AI-generated reply."
            ]
        ]


step : String -> String -> String -> Html Msg
step barClass title body =
    li [ class ("step " ++ barClass) ]
        [ h3 [] [ text title ]
        , p [] [ text body ]
        ]
