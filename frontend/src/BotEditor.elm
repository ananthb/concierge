module BotEditor exposing
    ( Config
    , Preset
    , validate
    , view
    )

{-| Shared bot configuration editor used in both the dashboard Channels tab and
the public live demo.
-}

import Api exposing (StaticRule)
import Html exposing (Html, button, div, input, label, p, span, text, textarea)
import Html.Attributes exposing (attribute, class, classList, for, id, name, placeholder, rows, type_, value)
import Html.Events exposing (onClick, onInput)


type alias Preset =
    { id : String
    , label : String
    , tag : String
    , subtitle : String
    , prompt : String
    , rules : List StaticRule
    }


type alias Config =
    { botPrompt : String
    , rules : List StaticRule
    }


maxRules : Int
maxRules =
    10


{-| Client-side validation matching the backend limits. Returns `Just message`
on the first invalid field, or `Nothing` when ready to save/send.
-}
validate : { a | botPrompt : String, rules : List StaticRule } -> Maybe String
validate config =
    if String.length config.botPrompt > 600 then
        Just "Bot persona prompt must be 600 characters or fewer."

    else if List.length config.rules > maxRules then
        Just "You can add up to 10 static rules."

    else
        validateRules 1 config.rules


validateRules : Int -> List StaticRule -> Maybe String
validateRules index rules =
    case rules of
        [] ->
            Nothing

        rule :: rest ->
            let
                prefix =
                    "Rule " ++ String.fromInt index ++ ": "
            in
            if String.isEmpty (String.trim rule.matchRule) then
                Just (prefix ++ "Write when to send (match rule).")

            else if String.length rule.matchRule > 300 then
                Just (prefix ++ "Match rule must be 300 characters or fewer.")

            else if String.isEmpty (String.trim rule.replyText) then
                Just (prefix ++ "Write the static reply.")

            else if String.length rule.replyText > 2000 then
                Just (prefix ++ "Static reply must be 2000 characters or fewer.")

            else
                validateRules (index + 1) rest


view :
    { idPrefix : String
    , presets : List Preset
    , config : Config
    , onSelectPreset : Preset -> msg
    , onChange : Config -> msg
    }
    -> Html msg
view args =
    div [ class "bot-editor" ]
        [ presetsView args
        , promptField args
        , rulesSection args
        ]


presetsView :
    { idPrefix : String
    , presets : List Preset
    , config : Config
    , onSelectPreset : Preset -> msg
    , onChange : Config -> msg
    }
    -> Html msg
presetsView args =
    if List.isEmpty args.presets then
        text ""

    else
        div [ class "field" ]
            [ label [] [ text "Starter presets" ]
            , div [ class "persona-grid" ]
                (List.map (presetButton args.config args.onSelectPreset) args.presets)
            ]


presetButton : Config -> (Preset -> msg) -> Preset -> Html msg
presetButton config onSelectPreset preset =
    let
        isSelected =
            config.botPrompt
                == preset.prompt
                && config.rules
                == preset.rules
    in
    button
        [ classList
            [ ( "persona-btn", True )
            , ( "is-active", isSelected )
            ]
        , type_ "button"
        , onClick (onSelectPreset preset)
        , attribute "aria-pressed"
            (if isSelected then
                "true"

             else
                "false"
            )
        ]
        ([ div [ class "persona-head" ]
            [ span [ class "persona-label" ] [ text preset.label ]
            , span [ class "persona-tag" ] [ text preset.tag ]
            ]
         ]
            ++ (if String.isEmpty preset.subtitle then
                    []

                else
                    [ p [ class "persona-desc" ] [ text preset.subtitle ] ]
               )
        )


promptField :
    { a
        | idPrefix : String
        , config : Config
        , onChange : Config -> msg
    }
    -> Html msg
promptField args =
    let
        config =
            args.config

        promptId =
            args.idPrefix ++ "-bot-prompt"

        promptLen =
            String.length config.botPrompt

        promptError =
            if promptLen > 600 then
                Just "Bot persona prompt must be 600 characters or fewer."

            else
                Nothing
    in
    div [ class "field" ]
        [ label [ for promptId ] [ text "Bot persona prompt (up to 600 chars)" ]
        , textarea
            [ id promptId
            , name promptId
            , rows 4
            , value config.botPrompt
            , onInput (\v -> args.onChange { config | botPrompt = v })
            ]
            []
        , case promptError of
            Just err ->
                p [ class "field-error" ] [ text err ]

            Nothing ->
                text ""
        , p [ class "hint" ]
            [ text
                (String.fromInt promptLen
                    ++ "/600 chars · All bots share the same system prompt and hand off to a human quickly."
                )
            ]
        ]


rulesSection :
    { a
        | idPrefix : String
        , config : Config
        , onChange : Config -> msg
    }
    -> Html msg
rulesSection args =
    let
        config =
            args.config

        ruleCount =
            List.length config.rules
    in
    div [ class "field" ]
        [ label [] [ text "Optional static rules" ]
        , p [ class "hint" ]
            [ text "Instant answers for opening hours, appointment times, calendar links, or policies. Anything else falls back to your bot persona." ]
        , if List.isEmpty config.rules then
            p [ class "muted" ]
                [ text "No static rules added. All replies will use your bot persona." ]

          else
            div [ class "rule-list" ]
                (List.indexedMap (ruleCard args) config.rules)
        , if ruleCount < maxRules then
            button
                [ class "btn"
                , type_ "button"
                , onClick
                    (args.onChange
                        { config
                            | rules = config.rules ++ [ { matchRule = "", replyText = "" } ]
                        }
                    )
                ]
                [ text "+ Add static rule" ]

          else
            p [ class "hint" ] [ text "Maximum of 10 static rules reached." ]
        ]


ruleCard :
    { a
        | idPrefix : String
        , config : Config
        , onChange : Config -> msg
    }
    -> Int
    -> StaticRule
    -> Html msg
ruleCard args idx rule =
    let
        config =
            args.config

        matchId =
            args.idPrefix ++ "-rule-" ++ String.fromInt idx ++ "-match"

        replyId =
            args.idPrefix ++ "-rule-" ++ String.fromInt idx ++ "-reply"

        matchLen =
            String.length rule.matchRule

        replyLen =
            String.length rule.replyText

        matchError =
            if matchLen > 300 then
                Just "Match rule must be 300 characters or fewer."

            else
                Nothing

        replyError =
            if replyLen > 2000 then
                Just "Static reply must be 2000 characters or fewer."

            else
                Nothing

        updateAt f =
            args.onChange
                { config
                    | rules =
                        List.indexedMap
                            (\i r ->
                                if i == idx then
                                    f r

                                else
                                    r
                            )
                            config.rules
                }

        removeAt =
            args.onChange
                { config
                    | rules =
                        List.indexedMap Tuple.pair config.rules
                            |> List.filter (\( i, _ ) -> i /= idx)
                            |> List.map Tuple.second
                }
    in
    div [ class "rule-card" ]
        [ div [ class "rule-card-head" ]
            [ span [] [ text ("Static rule #" ++ String.fromInt (idx + 1)) ]
            , button
                [ class "btn btn-ghost"
                , type_ "button"
                , onClick removeAt
                ]
                [ text "Remove" ]
            ]
        , div [ class "field" ]
            [ label [ for matchId ] [ text "When to send (match rule)" ]
            , input
                [ id matchId
                , name matchId
                , type_ "text"
                , value rule.matchRule
                , placeholder "e.g. Customer asks for opening hours or booking link"
                , onInput (\v -> updateAt (\r -> { r | matchRule = v }))
                ]
                []
            , case matchError of
                Just err ->
                    p [ class "field-error" ] [ text err ]

                Nothing ->
                    text ""
            , p [ class "hint" ] [ text (String.fromInt matchLen ++ "/300 chars") ]
            ]
        , div [ class "field" ]
            [ label [ for replyId ] [ text "Static reply" ]
            , textarea
                [ id replyId
                , name replyId
                , rows 3
                , value rule.replyText
                , placeholder "e.g. Open Mon–Sat 9:30 AM – 7:30 PM. Book a slot: https://cal.example.com/..."
                , onInput (\v -> updateAt (\r -> { r | replyText = v }))
                ]
                []
            , case replyError of
                Just err ->
                    p [ class "field-error" ] [ text err ]

                Nothing ->
                    text ""
            , p [ class "hint" ] [ text (String.fromInt replyLen ++ "/2000 chars") ]
            ]
        ]
