module Main exposing (main)

{-| Application entry point: one bootstrap request, then routing.

The load sequence is deliberately one round-trip. `/api/bootstrap` answers with
pricing, the demo config and persona catalog, and the session if there is one —
so the first frame after the spinner is the real page, not a skeleton that
fills in four times.

**Where the app sends you** is decided in exactly one place: the session's
`destination` field, computed server-side from whether onboarding is sealed.
Landing on `/dashboard` mid-wizard bounces to `/wizard`, and the reverse once
setup is done. Neither the worker's redirects nor this module duplicates that
rule.

A 401 from any request means the cookie is gone. The app drops its session and
renders the login screen rather than showing an error, because "signed out" is
a state, not a failure.

-}

import Api
import Browser
import Browser.Navigation as Nav
import Html exposing (Html, div, main_, text)
import Html.Attributes exposing (class)
import Page.Dashboard as Dashboard
import Page.Landing as Landing
import Page.Static as Static
import Page.Wizard as Wizard
import RemoteData exposing (RemoteData(..))
import Route exposing (Route)
import Ui
import Url exposing (Url)


main : Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }


type alias Flags =
    { width : Int }



-- MODEL


type alias Model =
    { key : Nav.Key
    , route : Route
    , boot : Api.Data Api.Bootstrap

    -- Kept alongside `boot` so signing out doesn't require re-fetching
    -- everything else in the bootstrap payload.
    , session : Maybe Api.Session
    , page : Page
    }


{-| Per-route state. Routes with no state of their own don't appear here.
-}
type Page
    = LandingPage Landing.Model
    | WizardPage Wizard.Model
    | DashboardPage Dashboard.Model
    | Stateless


init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init _ url key =
    ( { key = key
      , route = Route.fromUrl url
      , boot = Loading
      , session = Nothing
      , page = Stateless
      }
    , Api.getBootstrap GotBootstrap
    )



-- UPDATE


type Msg
    = UrlChanged Url
    | LinkClicked Browser.UrlRequest
    | GotBootstrap (Api.Data Api.Bootstrap)
    | LandingMsg Landing.Msg
    | WizardMsg Wizard.Msg
    | DashboardMsg Dashboard.Msg


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        LinkClicked request ->
            case request of
                Browser.Internal url ->
                    ( model, Nav.pushUrl model.key (Url.toString url) )

                Browser.External href ->
                    ( model, Nav.load href )

        UrlChanged url ->
            enterRoute (Route.fromUrl url) model

        GotBootstrap result ->
            let
                session =
                    case result of
                        Success boot ->
                            boot.session

                        _ ->
                            Nothing
            in
            -- Enter the route only now: until the session is known we can't
            -- tell whether /dashboard should render or redirect to /wizard.
            enterRoute model.route { model | boot = result, session = session }

        LandingMsg sub ->
            case ( model.page, model.boot ) of
                ( LandingPage landing, Success boot ) ->
                    let
                        ( updated, cmd ) =
                            Landing.update boot.demo sub landing
                    in
                    ( { model | page = LandingPage updated }, Cmd.map LandingMsg cmd )

                _ ->
                    ( model, Cmd.none )

        WizardMsg sub ->
            case model.page of
                WizardPage wizard ->
                    let
                        ( updated, cmd ) =
                            Wizard.update sub wizard
                    in
                    -- Finishing setup flips `completed`, which changes where
                    -- the app should be. Follow it rather than leaving the
                    -- user on a sealed wizard.
                    case updated.wizard of
                        Success w ->
                            if w.completed then
                                ( { model
                                    | page = WizardPage updated
                                    , session = Maybe.map markOnboarded model.session
                                  }
                                , Nav.pushUrl model.key (Route.toPath Route.Dashboard)
                                )

                            else
                                ( { model | page = WizardPage updated }, Cmd.map WizardMsg cmd )

                        Failure err ->
                            if Api.isUnauthenticated err then
                                signedOut model

                            else
                                ( { model | page = WizardPage updated }, Cmd.map WizardMsg cmd )

                        _ ->
                            ( { model | page = WizardPage updated }, Cmd.map WizardMsg cmd )

                _ ->
                    ( model, Cmd.none )

        DashboardMsg sub ->
            case model.page of
                DashboardPage dash ->
                    let
                        ( updated, cmd ) =
                            Dashboard.update sub dash
                    in
                    ( { model | page = DashboardPage updated }, Cmd.map DashboardMsg cmd )

                _ ->
                    ( model, Cmd.none )


markOnboarded : Api.Session -> Api.Session
markOnboarded session =
    { session | destination = "dashboard", onboardingStep = Nothing }


{-| Forget the session and show the login page. Called on any 401.
-}
signedOut : Model -> ( Model, Cmd Msg )
signedOut model =
    ( { model | session = Nothing, page = Stateless, route = Route.Login }
    , Nav.pushUrl model.key (Route.toPath Route.Login)
    )


{-| Set up the page for a route, applying the two gates that depend on session
state: signed-in-only routes, and the wizard/dashboard split.
-}
enterRoute : Route -> Model -> ( Model, Cmd Msg )
enterRoute route model =
    let
        settled =
            { model | route = route }
    in
    case route of
        Route.Landing ->
            ( { settled | page = LandingPage Landing.init }, Cmd.none )

        Route.Wizard ->
            case model.session of
                Nothing ->
                    needsSignIn settled

                Just session ->
                    if session.destination == "dashboard" then
                        redirectTo Route.Dashboard settled

                    else
                        let
                            ( wizard, cmd ) =
                                Wizard.init
                        in
                        ( { settled | page = WizardPage wizard }, Cmd.map WizardMsg cmd )

        Route.Dashboard ->
            dashboardRoute Dashboard.Overview settled

        Route.DashboardChannels ->
            dashboardRoute Dashboard.Channels settled

        Route.DashboardPersona ->
            dashboardRoute Dashboard.Persona settled

        Route.DashboardBilling ->
            dashboardRoute Dashboard.Billing settled

        Route.DashboardSettings ->
            dashboardRoute Dashboard.Settings settled

        _ ->
            ( { settled | page = Stateless }, Cmd.none )


dashboardRoute : Dashboard.Tab -> Model -> ( Model, Cmd Msg )
dashboardRoute tab model =
    case model.session of
        Nothing ->
            needsSignIn model

        Just session ->
            if session.destination == "wizard" then
                redirectTo Route.Wizard model

            else
                case model.page of
                    -- Switching tabs keeps whatever's already loaded.
                    DashboardPage existing ->
                        let
                            ( updated, cmd ) =
                                Dashboard.switchTab tab existing
                        in
                        ( { model | page = DashboardPage updated }, Cmd.map DashboardMsg cmd )

                    _ ->
                        let
                            ( dash, cmd ) =
                                Dashboard.init tab
                        in
                        ( { model | page = DashboardPage dash }, Cmd.map DashboardMsg cmd )


needsSignIn : Model -> ( Model, Cmd Msg )
needsSignIn model =
    ( { model | page = Stateless, route = Route.Login }, Cmd.none )


redirectTo : Route -> Model -> ( Model, Cmd Msg )
redirectTo route model =
    ( model, Nav.replaceUrl model.key (Route.toPath route) )


subscriptions : Model -> Sub Msg
subscriptions model =
    case ( model.page, model.boot ) of
        ( LandingPage landing, Success boot ) ->
            Sub.map LandingMsg (Landing.subscriptions boot.demo landing)

        ( DashboardPage _, _ ) ->
            Sub.map DashboardMsg Dashboard.subscriptions

        _ ->
            Sub.none



-- VIEW


view : Model -> Browser.Document Msg
view model =
    { title = title model.route
    , body =
        [ Ui.header model.session
        , main_ [ class "site-main" ] [ content model ]
        , Ui.footer
        ]
    }


title : Route -> String
title route =
    case route of
        Route.Landing ->
            "Concierge — automatic WhatsApp replies for small businesses"

        Route.Pricing ->
            "Pricing — Concierge"

        Route.Features ->
            "Features — Concierge"

        Route.Terms ->
            "Terms of Service — Concierge"

        Route.Privacy ->
            "Privacy Policy — Concierge"

        Route.Login ->
            "Sign in — Concierge"

        Route.Wizard ->
            "Set up Concierge"

        Route.Manage ->
            "Operations — Concierge"

        Route.NotFound ->
            "Not found — Concierge"

        _ ->
            "Dashboard — Concierge"


content : Model -> Html Msg
content model =
    -- Everything needs the bootstrap payload, so one spinner here covers the
    -- whole app rather than each page growing its own.
    Ui.remote model.boot <|
        \boot ->
            case ( model.route, model.page ) of
                ( Route.Landing, LandingPage landing ) ->
                    Html.map LandingMsg (Landing.view boot.pricing boot.demo landing)

                ( Route.Pricing, _ ) ->
                    Static.pricing boot.pricing

                ( Route.Features, _ ) ->
                    Static.features

                ( Route.Terms, _ ) ->
                    Static.terms

                ( Route.Privacy, _ ) ->
                    Static.privacy

                ( Route.Login, _ ) ->
                    Static.login

                ( Route.Wizard, WizardPage wizard ) ->
                    Html.map WizardMsg (Wizard.view wizard)

                ( _, DashboardPage dash ) ->
                    case model.session of
                        Just session ->
                            Html.map DashboardMsg (Dashboard.view session dash)

                        Nothing ->
                            Static.login

                ( Route.Manage, _ ) ->
                    -- The operator console is API-only for now: /api/manage/*
                    -- is complete but has no UI. Say so instead of rendering
                    -- an empty page.
                    div [ class "page" ]
                        [ text "The operations console has no interface yet. Its endpoints live under /api/manage/." ]

                _ ->
                    Static.notFound
