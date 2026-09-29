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
import Page.Manage as Manage
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
    | ManagePage Manage.Model
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
    | ManageMsg Manage.Msg


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

        ManageMsg sub ->
            case model.page of
                ManagePage console ->
                    let
                        ( updated, cmd ) =
                            Manage.update sub console
                    in
                    ( { model | page = ManagePage updated }, Cmd.map ManageMsg cmd )

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

        -- No session check: the credential here is the Access JWT, which
        -- this app can't see. The page asks the API and renders whatever it
        -- says, including the refusal.
        Route.Manage ->
            case model.page of
                ManagePage _ ->
                    ( settled, Cmd.none )

                _ ->
                    let
                        ( console, cmd ) =
                            Manage.init
                    in
                    ( { settled | page = ManagePage console }, Cmd.map ManageMsg cmd )

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
    case model.route of
        -- Routes with no data dependency. Rendered without waiting on
        -- /api/bootstrap, so they appear on first paint and so a prerendered
        -- snapshot of them carries real copy rather than a spinner.
        Route.Features ->
            Static.features

        Route.Terms ->
            Static.terms

        Route.Privacy ->
            Static.privacy

        Route.Login ->
            Static.login

        Route.NotFound ->
            Static.notFound

        Route.Manage ->
            case model.page of
                ManagePage console ->
                    Html.map ManageMsg (Manage.view console)

                _ ->
                    text ""

        -- Everything below needs the payload. The copy on these pages still
        -- renders immediately; only the values wait.
        Route.Pricing ->
            Static.pricing (RemoteData.map .pricing model.boot)

        Route.Landing ->
            case model.page of
                LandingPage landing ->
                    Html.map LandingMsg
                        (Landing.view
                            (RemoteData.map .pricing model.boot)
                            (RemoteData.map .demo model.boot)
                            landing
                        )

                _ ->
                    Ui.spinner

        _ ->
            -- The signed-in app. Gated on a session, which only arrives with
            -- the bootstrap payload, so this genuinely can't render early.
            Ui.remote model.boot <|
                \_ ->
                    case ( model.route, model.page ) of
                        ( Route.Wizard, WizardPage wizard ) ->
                            Html.map WizardMsg (Wizard.view wizard)

                        ( _, DashboardPage dash ) ->
                            case model.session of
                                Just session ->
                                    Html.map DashboardMsg (Dashboard.view session dash)

                                Nothing ->
                                    Static.login

                        _ ->
                            Static.notFound
