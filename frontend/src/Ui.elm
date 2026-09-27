module Ui exposing
    ( banner
    , button
    , card
    , field
    , footer
    , header
    , linkButton
    , remote
    , sectionTitle
    , spinner
    , textarea
    , toggle
    )

{-| Shared view pieces.

Class names match the existing stylesheets in `public/css/` — this refactor
replaced the templates that rendered the markup, not the design system, so
`.btn`, `.card`, `.field` and friends are unchanged.

-}

import Api
import Html exposing (Html, a, div, h2, input, label, li, nav, p, span, text, ul)
import Html.Attributes exposing (attribute, class, classList, disabled, for, href, id, name, placeholder, rows, type_, value)
import Html.Events exposing (onCheck, onClick, onInput)
import RemoteData exposing (RemoteData(..))
import Route exposing (Route)


{-| Render remote data, handling all four states in one place so no page
forgets a loading spinner or swallows an error.

`NotAsked` and `Loading` both show the spinner: from the user's side there is
no difference between "about to ask" and "asking", and distinguishing them
just produces a blank flash before the spinner appears.

-}
remote : Api.Data a -> (a -> Html msg) -> Html msg
remote data toHtml =
    case data of
        NotAsked ->
            spinner

        Loading ->
            spinner

        Failure err ->
            banner "error" (Api.errorMessage err)

        Success value ->
            toHtml value


spinner : Html msg
spinner =
    div [ class "spinner", attribute "role" "status", attribute "aria-label" "Loading" ] []


{-| A message banner. `kind` is "error", "success" or "info".
-}
banner : String -> String -> Html msg
banner kind message =
    div
        [ class kind
        , class "banner"
        , attribute "role"
            (if kind == "error" then
                "alert"

             else
                "status"
            )
        ]
        [ text message ]


card : List (Html msg) -> Html msg
card =
    div [ class "card" ]


sectionTitle : String -> Html msg
sectionTitle title =
    h2 [ class "section-title" ] [ text title ]


button : { label : String, onClick : msg, primary : Bool, busy : Bool } -> Html msg
button config =
    Html.button
        [ classList
            [ ( "btn", True )
            , ( "btn-primary", config.primary )
            , ( "btn-secondary", not config.primary )
            , ( "is-busy", config.busy )
            ]
        , onClick config.onClick
        , disabled config.busy
        , type_ "button"
        ]
        [ text
            (if config.busy then
                "Working…"

             else
                config.label
            )
        ]


linkButton : { label : String, url : String, primary : Bool } -> Html msg
linkButton config =
    a
        [ href config.url
        , classList
            [ ( "btn", True )
            , ( "btn-primary", config.primary )
            , ( "btn-secondary", not config.primary )
            ]
        ]
        [ text config.label ]


{-| A labelled text input. `hint` renders under the control; pass "" to omit.
-}
field :
    { id : String
    , label : String
    , value : String
    , hint : String
    , required : Bool
    , onInput : String -> msg
    }
    -> Html msg
field config =
    div [ class "field" ]
        [ label [ for config.id ]
            [ text config.label
            , if config.required then
                span [ class "required" ] [ text " *" ]

              else
                text ""
            ]
        , input
            [ id config.id
            , name config.id
            , type_ "text"
            , value config.value
            , onInput config.onInput
            ]
            []
        , if String.isEmpty config.hint then
            text ""

          else
            p [ class "hint" ] [ text config.hint ]
        ]


textarea :
    { id : String
    , label : String
    , value : String
    , hint : String
    , rows : Int
    , onInput : String -> msg
    }
    -> Html msg
textarea config =
    div [ class "field" ]
        [ label [ for config.id ] [ text config.label ]
        , Html.textarea
            [ id config.id
            , name config.id
            , rows config.rows
            , value config.value
            , onInput config.onInput
            ]
            []
        , if String.isEmpty config.hint then
            text ""

          else
            p [ class "hint" ] [ text config.hint ]
        ]


toggle : { id : String, label : String, checked : Bool, onCheck : Bool -> msg } -> Html msg
toggle config =
    div [ class "field field-toggle" ]
        [ input
            [ id config.id
            , name config.id
            , type_ "checkbox"
            , Html.Attributes.checked config.checked
            , onCheck config.onCheck
            ]
            []
        , label [ for config.id ] [ text config.label ]
        ]


{-| Site header. `session` decides whether the nav offers sign-in or the app.
-}
header : Maybe Api.Session -> Html msg
header session =
    Html.header [ class "site-header" ]
        [ a [ Route.href Route.Landing, class "brand" ]
            [ Html.img [ Html.Attributes.src "/logo.svg", Html.Attributes.alt "", Html.Attributes.width 32, Html.Attributes.height 32 ] []
            , span [] [ text "Concierge" ]
            ]
        , nav [ class "site-nav" ]
            (case session of
                Nothing ->
                    [ navLink Route.Features "Features"
                    , navLink Route.Pricing "Pricing"
                    , a [ href "/auth/login", class "btn btn-primary" ] [ text "Sign in" ]
                    ]

                Just s ->
                    [ navLink Route.Features "Features"
                    , navLink Route.Pricing "Pricing"
                    , navLink
                        (if s.destination == "wizard" then
                            Route.Wizard

                         else
                            Route.Dashboard
                        )
                        (if s.destination == "wizard" then
                            "Finish setup"

                         else
                            "Dashboard"
                        )
                    ]
            )
        ]


navLink : Route -> String -> Html msg
navLink route label_ =
    a [ Route.href route, class "nav-link" ] [ text label_ ]


footer : Html msg
footer =
    Html.footer [ class "site-footer" ]
        [ ul [ class "footer-links" ]
            [ li [] [ a [ Route.href Route.Terms ] [ text "Terms" ] ]
            , li [] [ a [ Route.href Route.Privacy ] [ text "Privacy" ] ]
            , li [] [ a [ href "https://ananthb.github.io/concierge/" ] [ text "Docs" ] ]
            ]
        , p [ class "footer-note" ]
            [ text "Message bodies pass through but are never stored. Only channel, direction, sender, recipient and timestamp are kept." ]
        ]
