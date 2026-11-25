module Page.Landing exposing (Model, Msg(..), demoPage, init, subscriptions, update, view)

{-| The landing page and the interactive live demo.

The `/demo` route renders the shared production `BotEditor` setup screen in the
main column and a visually separate floating "Chat as if you're the customer"
widget in the bottom-right corner.

-}

import Api
import BotEditor
import Browser.Dom as Dom
import Dict
import Format
import Html exposing (Html, button, div, h1, h2, h3, input, label, li, p, section, span, text, ul)
import Html.Attributes exposing (attribute, class, classList, disabled, for, id, placeholder, type_, value)
import Html.Events exposing (onClick, onInput, onSubmit)
import Html.Keyed as Keyed
import RemoteData exposing (RemoteData(..))
import Route
import Task
import Time
import Ui



-- MODEL


type alias Model =
    { demo : Maybe DemoState

    -- Seconds since the visitor last typed or sent. Drives the silent idle reset.
    , idleSeconds : Int
    , reducedMotion : Bool
    , wordIndex : Int
    , phase : TypePhase
    }


type WidgetState
    = WidgetDefault
    | WidgetOpen
    | WidgetMinimized


type alias DemoState =
    { personaId : String
    , businessName : String
    , config : BotEditor.Config
    , messages : List Turn
    , draft : String

    -- The in-flight reply. `Loading` renders the typing indicator.
    , pending : Api.Data Api.DemoReply
    , handoff : Bool
    , validationError : Maybe String
    , safetyError : Maybe String
    , widget : WidgetState
    , maxUserTurns : Int
    , idleTimeoutSecs : Int
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


type alias Turn =
    { role : String
    , content : String
    }


init : Bool -> Maybe Api.DemoConfig -> Model
init reducedMotion maybeConfig =
    let
        initialDemo =
            maybeConfig
                |> Maybe.andThen
                    (\cfg ->
                        if cfg.enabled then
                            List.head cfg.personas
                                |> Maybe.map (initDemoState cfg)

                        else
                            Nothing
                    )
    in
    { demo = initialDemo
    , idleSeconds = 0
    , reducedMotion = reducedMotion
    , wordIndex = 0
    , phase = Holding 34
    }


initDemoState : Api.DemoConfig -> Api.DemoPersona -> DemoState
initDemoState cfg persona =
    { personaId = persona.id
    , businessName = persona.businessName
    , config =
        { mode = persona.mode
        , botPrompt = persona.prompt
        , matchRule = persona.matchRule
        , text = persona.cannedText
        }
    , messages = []
    , draft = ""
    , pending = NotAsked
    , handoff = False
    , validationError = Nothing
    , safetyError = Nothing
    , widget = WidgetDefault
    , maxUserTurns = cfg.maxUserTurns
    , idleTimeoutSecs = cfg.idleTimeoutSecs
    }



-- UPDATE


type Msg
    = SelectPreset BotEditor.Preset
    | SetBusinessName String
    | UpdateBotConfig BotEditor.Config
    | OpenWidget
    | MinimizeWidget
    | Draft String
    | Send
    | GotReply (Api.Data Api.DemoReply)
    | ResetChat
    | Tick
    | TypeTick
    | ReducedSwapTick
    | LoginClicked
    | NoOp


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SelectPreset preset ->
            ( { model
                | demo =
                    Maybe.map
                        (\d ->
                            { d
                                | personaId = preset.id
                                , businessName = preset.subtitle
                                , config =
                                    { mode = preset.mode
                                    , botPrompt = preset.prompt
                                    , matchRule = preset.matchRule
                                    , text = preset.cannedText
                                    }
                                , messages = []
                                , pending = NotAsked
                                , handoff = False
                                , validationError = Nothing
                                , safetyError = Nothing
                            }
                        )
                        model.demo
                , idleSeconds = 0
              }
            , Cmd.none
            )

        SetBusinessName name ->
            ( { model
                | demo =
                    Maybe.map
                        (\d ->
                            { d
                                | businessName = name
                                , validationError = Nothing
                                , safetyError = Nothing
                            }
                        )
                        model.demo
                , idleSeconds = 0
              }
            , Cmd.none
            )

        UpdateBotConfig cfg ->
            ( { model
                | demo =
                    Maybe.map
                        (\d ->
                            { d
                                | config = cfg
                                , validationError = Nothing
                                , safetyError = Nothing
                            }
                        )
                        model.demo
                , idleSeconds = 0
              }
            , Cmd.none
            )

        OpenWidget ->
            ( { model
                | demo = Maybe.map (\d -> { d | widget = WidgetOpen }) model.demo
              }
            , focusInput
            )

        MinimizeWidget ->
            ( { model
                | demo = Maybe.map (\d -> { d | widget = WidgetMinimized }) model.demo
              }
            , Cmd.none
            )

        Draft text_ ->
            ( { model
                | demo = Maybe.map (\d -> { d | draft = text_ }) model.demo
                , idleSeconds = 0
              }
            , Cmd.none
            )

        Send ->
            case model.demo of
                Just d ->
                    let
                        body =
                            String.trim d.draft
                    in
                    if String.isEmpty body || RemoteData.isLoading d.pending then
                        ( model, Cmd.none )

                    else
                        case BotEditor.validate d.config of
                            Just err ->
                                ( { model
                                    | demo =
                                        Just
                                            { d
                                                | validationError = Just err
                                                , safetyError = Nothing
                                            }
                                  }
                                , Cmd.none
                                )

                            Nothing ->
                                let
                                    userTurns =
                                        d.messages
                                            |> List.filter (\m -> m.role == "user")
                                            |> List.length

                                    startFresh =
                                        userTurns >= d.maxUserTurns

                                    priorHistory =
                                        if startFresh then
                                            []

                                        else
                                            d.messages

                                    currentHandoff =
                                        if startFresh then
                                            False

                                        else
                                            d.handoff

                                    withUserTurn =
                                        priorHistory ++ [ { role = "user", content = body } ]
                                in
                                ( { model
                                    | demo =
                                        Just
                                            { d
                                                | messages = withUserTurn
                                                , draft = ""
                                                , pending = Loading
                                                , handoff = currentHandoff
                                                , validationError = Nothing
                                                , safetyError = Nothing
                                            }
                                    , idleSeconds = 0
                                  }
                                , Cmd.batch
                                    [ Api.demoChat
                                        { personaId = d.personaId
                                        , businessName = d.businessName
                                        , mode = d.config.mode
                                        , botPrompt = d.config.botPrompt
                                        , matchRule = d.config.matchRule
                                        , cannedText = d.config.text
                                        , history = priorHistory
                                        , message = body
                                        , handoff = currentHandoff
                                        }
                                        GotReply
                                    , scrollToBottom
                                    ]
                                )

                Nothing ->
                    ( model, Cmd.none )

        GotReply result ->
            case model.demo of
                Just d ->
                    let
                        updated =
                            case result of
                                Success reply ->
                                    { d
                                        | messages =
                                            d.messages
                                                ++ [ { role = "assistant", content = reply.reply } ]
                                        , pending = NotAsked
                                        , handoff = d.handoff || reply.handedOff
                                        , safetyError = Nothing
                                    }

                                Failure (Api.Fault { code, message }) ->
                                    if code == "unsafe_content" then
                                        { d
                                            | pending = result
                                            , safetyError = Just message
                                        }

                                    else
                                        { d | pending = result }

                                _ ->
                                    { d | pending = result }
                    in
                    ( { model | demo = Just updated }, scrollToBottom )

                Nothing ->
                    ( model, Cmd.none )

        ResetChat ->
            ( { model
                | demo =
                    Maybe.map
                        (\d ->
                            { d
                                | messages = []
                                , draft = ""
                                , pending = NotAsked
                                , handoff = False
                                , validationError = Nothing
                                , safetyError = Nothing
                            }
                        )
                        model.demo
                , idleSeconds = 0
              }
            , Cmd.none
            )

        Tick ->
            case model.demo of
                Just d ->
                    let
                        nextIdle =
                            model.idleSeconds + 1
                    in
                    if nextIdle >= d.idleTimeoutSecs then
                        ( { model
                            | demo =
                                Just
                                    { d
                                        | messages = []
                                        , pending = NotAsked
                                        , handoff = False
                                    }
                            , idleSeconds = 0
                          }
                        , Cmd.none
                        )

                    else
                        ( { model | idleSeconds = nextIdle }, Cmd.none )

                Nothing ->
                    ( model, Cmd.none )

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


focusInput : Cmd Msg
focusInput =
    Task.attempt (\_ -> NoOp) (Dom.focus "demo-input")


scrollToBottom : Cmd Msg
scrollToBottom =
    Dom.getViewportOf "demo-transcript"
        |> Task.andThen (\info -> Dom.setViewportOf "demo-transcript" 0 info.scene.height)
        |> Task.attempt (\_ -> NoOp)


{-| Tick the hero word cycler and, when a demo conversation has messages, the
silent idle reset timer.
-}
subscriptions : Maybe Api.DemoConfig -> Model -> Sub Msg
subscriptions maybeConfig model =
    let
        idleSub =
            case ( maybeConfig, model.demo ) of
                ( Just _, Just d ) ->
                    if not (List.isEmpty d.messages) || d.handoff then
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
            [ text "One bot prompt, fast human handoff, 1 credit = 1 reply." ]
        , div [ class "hero-actions" ]
            [ button
                [ type_ "button"
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
            [ featureCard "AI-generated replies"
                "One built-in bot prompt handles everyday customer questions and hands off to a human when it shouldn't decide alone."
            , featureCard "Static replies for simple questions"
                "Set a natural-language match rule for common questions. High-confidence matches send your preset reply; anything uncertain falls back to AI."
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
            [ text "1 credit = 1 WhatsApp reply. No subscriptions, no per-seat fees." ]
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
demoPage _ demo model =
    div [ class "page page-demo" ]
        [ div [ class "demo-setup" ]
            [ h1 [] [ text "Live demo" ]
            , p [ class "demo-lead" ]
                [ text "Configure a reply bot using the same setup screen as the app, then test it as a customer." ]
            , case demo of
                Success config ->
                    if config.enabled && not (List.isEmpty config.personas) then
                        case model.demo of
                            Just d ->
                                demoWorkspace config d

                            Nothing ->
                                p [ class "muted" ] [ text "The live demo is currently unavailable." ]

                    else
                        p [ class "muted" ] [ text "The live demo is currently unavailable." ]

                Loading ->
                    Ui.spinner

                _ ->
                    text ""
            ]
        ]


demoWorkspace : Api.DemoConfig -> DemoState -> Html Msg
demoWorkspace config d =
    let
        presets =
            List.map toBotEditorPreset config.personas
    in
    div []
        [ Ui.colorCard "color-bar-green"
            [ case d.safetyError of
                Just err ->
                    Ui.banner "error" err

                Nothing ->
                    case d.validationError of
                        Just err ->
                            Ui.banner "error" err

                        Nothing ->
                            text ""
            , div [ class "field" ]
                [ label [ for "demo-business-name" ] [ text "Business name" ]
                , input
                    [ id "demo-business-name"
                    , type_ "text"
                    , value d.businessName
                    , placeholder "Your business name"
                    , onInput SetBusinessName
                    ]
                    []
                ]
            , BotEditor.view
                { idPrefix = "demo"
                , presets = presets
                , config = d.config
                , onSelectPreset = SelectPreset
                , onChange = UpdateBotConfig
                }
            ]
        , customerChatDock d
        ]


toBotEditorPreset : Api.DemoPersona -> BotEditor.Preset
toBotEditorPreset persona =
    { id = persona.id
    , label = persona.label
    , tag = persona.tag
    , subtitle = persona.businessName
    , mode = persona.mode
    , prompt = persona.prompt
    , matchRule = persona.matchRule
    , cannedText = persona.cannedText
    }


customerChatDock : DemoState -> Html Msg
customerChatDock d =
    let
        dockClass =
            case d.widget of
                WidgetDefault ->
                    "demo-dock"

                WidgetOpen ->
                    "demo-dock is-open"

                WidgetMinimized ->
                    "demo-dock is-minimized"

        launcherClass =
            case d.widget of
                WidgetDefault ->
                    "demo-launcher"

                WidgetOpen ->
                    "demo-launcher is-hidden"

                WidgetMinimized ->
                    "demo-launcher is-visible"
    in
    div []
        [ section
            [ class dockClass
            , attribute "aria-label" "Chat as if you're the customer"
            ]
            [ div [ class "demo-widget" ]
                [ div [ class "demo-widget-head" ]
                    [ div [ class "demo-widget-title" ]
                        [ span [ class "demo-widget-kicker" ] [ text "Chat as if you're the customer" ]
                        , h2 [] [ text (displayBusinessName d.businessName) ]
                        ]
                    , div [ class "demo-widget-actions" ]
                        [ button
                            [ type_ "button"
                            , class "btn btn-ghost demo-widget-btn"
                            , onClick ResetChat
                            ]
                            [ text "Reset" ]
                        , button
                            [ type_ "button"
                            , class "btn btn-ghost demo-widget-btn"
                            , attribute "aria-label" "Minimize customer chat"
                            , onClick MinimizeWidget
                            ]
                            [ text "Minimize" ]
                        ]
                    ]
                , div [ class "demo-widget-status" ]
                    [ if d.handoff then
                        span [ class "pill pill-warn" ] [ text "Handed to a human" ]

                      else
                        span [ class "pill" ] [ text "Replying automatically" ]
                    ]
                , div [ class "demo-transcript", id "demo-transcript" ]
                    (if List.isEmpty d.messages && not (RemoteData.isLoading d.pending) then
                        [ p [ class "demo-empty" ]
                            [ text "Send a message as a customer to test your bot." ]
                        , pendingBubble d.pending
                        ]

                     else
                        List.map bubble d.messages
                            ++ [ pendingBubble d.pending ]
                    )
                , composer d
                ]
            ]
        , button
            [ type_ "button"
            , class launcherClass
            , onClick OpenWidget
            ]
            [ span [ class "demo-launcher-compact" ] [ text "Customer chat" ]
            , span [ class "demo-launcher-full" ] [ text "Chat as if you're the customer" ]
            ]
        ]


displayBusinessName : String -> String
displayBusinessName name =
    let
        trimmed =
            String.trim name
    in
    if String.isEmpty trimmed then
        "Your business"

    else
        trimmed


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


composer : DemoState -> Html Msg
composer d =
    Html.form [ class "demo-composer", onSubmit Send ]
        [ input
            [ id "demo-input"
            , class "demo-input"
            , placeholder "Ask them something…"
            , value d.draft
            , onInput Draft
            , attribute "autocomplete" "off"
            , Html.Attributes.maxlength 300
            ]
            []
        , button
            [ class "btn btn-primary"
            , type_ "submit"
            , disabled (String.isEmpty (String.trim d.draft) || RemoteData.isLoading d.pending)
            ]
            [ text "Send" ]
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
                "Choose a starter bot prompt or write your own (up to 600 chars), with optional rule-matched replies."
            , step "color-bar-amber"
                "3. 1 credit = 1 reply"
                "Pay as you go with prepaid credits."
            ]
        ]


step : String -> String -> String -> Html msg
step barClass title body =
    li [ class ("step " ++ barClass) ]
        [ h3 [] [ text title ]
        , p [] [ text body ]
        ]
