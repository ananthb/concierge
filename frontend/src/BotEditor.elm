module BotEditor exposing
    ( Config
    , Preset
    , validate
    , view
    )

{-| Shared bot configuration editor used in both the dashboard Channels tab and
the public live demo.
-}

import Html exposing (Html, button, div, input, label, p, span, text, textarea)
import Html.Attributes exposing (attribute, class, classList, for, id, name, placeholder, rows, type_, value)
import Html.Events exposing (onClick, onInput)


type alias Preset =
    { id : String
    , label : String
    , tag : String
    , subtitle : String
    , mode : String
    , prompt : String
    , matchRule : String
    , cannedText : String
    }


type alias Config =
    { mode : String
    , botPrompt : String
    , matchRule : String
    , text : String
    }


{-| Client-side validation matching the backend limits. Returns `Just message`
on the first invalid field, or `Nothing` when ready to save/send.
-}
validate : { a | mode : String, botPrompt : String, matchRule : String, text : String } -> Maybe String
validate config =
    if config.mode == "canned" && String.isEmpty (String.trim config.text) then
        Just "Write the matched reply message."

    else if config.mode == "canned" && String.length config.text > 2000 then
        Just "Matched reply text must be 2000 characters or fewer."

    else if config.mode == "canned" && String.length config.matchRule > 300 then
        Just "Match rule must be 300 characters or fewer."

    else if String.length config.botPrompt > 600 then
        Just "Bot prompt must be 600 characters or fewer."

    else
        Nothing


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
        , modePicker args.config args.onChange
        , if args.config.mode == "canned" then
            cannedFields args

          else
            text ""
        , promptField args
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
            config.mode
                == preset.mode
                && config.botPrompt
                == preset.prompt
                && config.matchRule
                == preset.matchRule
                && config.text
                == preset.cannedText
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


modePicker : Config -> (Config -> msg) -> Html msg
modePicker config onChange =
    div [ class "field" ]
        [ label [] [ text "Reply mode" ]
        , div [ class "choice-row" ]
            (List.map
                (\( wire, label_ ) ->
                    button
                        [ classList
                            [ ( "choice", True )
                            , ( "is-selected", config.mode == wire )
                            ]
                        , type_ "button"
                        , attribute "aria-pressed"
                            (if config.mode == wire then
                                "true"

                             else
                                "false"
                            )
                        , onClick (onChange { config | mode = wire })
                        ]
                        [ text label_ ]
                )
                [ ( "ai", "AI reply" )
                , ( "canned", "Rule-matched reply" )
                ]
            )
        ]


cannedFields :
    { a
        | idPrefix : String
        , config : Config
        , onChange : Config -> msg
    }
    -> Html msg
cannedFields args =
    let
        config =
            args.config

        textId =
            args.idPrefix ++ "-reply-text"

        ruleId =
            args.idPrefix ++ "-match-rule"

        textLen =
            String.length config.text

        ruleLen =
            String.length config.matchRule

        textError =
            if String.isEmpty (String.trim config.text) then
                Just "Write the matched reply message."

            else if textLen > 2000 then
                Just "Matched reply text must be 2000 characters or fewer."

            else
                Nothing

        ruleError =
            if ruleLen > 300 then
                Just "Match rule must be 300 characters or fewer."

            else
                Nothing
    in
    div []
        [ div [ class "field" ]
            [ label [ for textId ] [ text "Matched reply text" ]
            , textarea
                [ id textId
                , name textId
                , rows 4
                , value config.text
                , onInput (\v -> args.onChange { config | text = v })
                ]
                []
            , case textError of
                Just err ->
                    p [ class "field-error" ] [ text err ]

                Nothing ->
                    text ""
            , p [ class "hint" ]
                [ text
                    (String.fromInt textLen
                        ++ "/2000 chars · Sent word for word whenever a customer message matches."
                    )
                ]
            ]
        , div [ class "field" ]
            [ label [ for ruleId ] [ text "Match rule (optional)" ]
            , input
                [ id ruleId
                , name ruleId
                , type_ "text"
                , value config.matchRule
                , placeholder "e.g. Customer asks for opening hours or address"
                , onInput (\v -> args.onChange { config | matchRule = v })
                ]
                []
            , case ruleError of
                Just err ->
                    p [ class "field-error" ] [ text err ]

                Nothing ->
                    text ""
            , p [ class "hint" ]
                [ text
                    (String.fromInt ruleLen
                        ++ "/300 chars · Leave blank to always send the matched reply, or describe when to match; uncertain messages fall back to AI."
                    )
                ]
            ]
        ]


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

        promptLabel =
            if config.mode == "canned" then
                "Fallback AI bot prompt (optional, up to 600 chars)"

            else
                "Bot prompt (optional, up to 600 chars)"

        promptError =
            if promptLen > 600 then
                Just "Bot prompt must be 600 characters or fewer."

            else
                Nothing
    in
    div [ class "field" ]
        [ label [ for promptId ] [ text promptLabel ]
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
