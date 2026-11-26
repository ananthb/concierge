module Main exposing (main)

{-| Application entry point: one bootstrap request, then routing.

The load sequence is deliberately one round-trip. `/api/bootstrap` answers with
pricing, the demo config and sample businesses, and the session if there is
one — so the first frame after the spinner is the real page, not a skeleton
that fills in four times.

**Where the app sends you** is decided in exactly one place: the session's
`destination` field, computed server-side from whether onboarding is sealed.
Landing on `/dashboard` mid-wizard bounces to `/wizard`, and the reverse once
setup is done.

A 401 from any request means the cookie is gone. The app drops its session and
renders the login screen rather than showing an error, because "signed out" is
a state, not a failure.

-}

import Api
import Browser
import Browser.Dom as Dom
import Browser.Navigation as Nav
import Html exposing (Html, main_)
import Html.Attributes exposing (class)
import Json.Decode as D
import Page.Dashboard as Dashboard
import Page.Landing as Landing
import Page.Static as Static
import Page.Wizard as Wizard
import Ports
import RemoteData exposing (RemoteData(..))
import Route exposing (Route)
import Task
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
    D.Value



-- MODEL


type alias Model =
    { key : Nav.Key
    , route : Route
    , boot : Api.Data Api.Bootstrap
    , session : Maybe Api.Session
    , page : Page
    , authBusy : Bool
    , authError : Maybe String
    , reducedMotion : Bool
    }


type Page
    = LandingPage Landing.Model
    | WizardPage Wizard.Model
    | DashboardPage Dashboard.Model
    | Stateless


init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        reducedMotion =
            D.decodeValue (D.field "reducedMotion" D.bool) flags
                |> Result.withDefault False

        urlErr =
            parseQueryError url.query
    in
    ( { key = key
      , route = Route.fromUrl url
      , boot = Loading
      , session = Nothing
      , page = Stateless
      , authBusy = False
      , authError = urlErr
      , reducedMotion = reducedMotion
      }
    , Api.getBootstrap GotBootstrap
    )


parseQueryError : Maybe String -> Maybe String
parseQueryError maybeQuery =
    case maybeQuery of
        Nothing ->
            Nothing

        Just q ->
            q
                |> String.split "&"
                |> List.filterMap
                    (\pair ->
                        case String.split "=" pair of
                            [ "error", code ] ->
                                let
                                    cleaned =
                                        Url.percentDecode code
                                            |> Maybe.withDefault code
                                            |> String.trim
                                in
                                if String.isEmpty cleaned then
                                    Nothing

                                else
                                    Just (errorCodeToMessage cleaned)

                            _ ->
                                Nothing
                    )
                |> List.head


errorCodeToMessage : String -> String
errorCodeToMessage code =
    case code of
        "missing_waba" ->
            "No WhatsApp Business Account was found on your Meta account. Make sure your Meta Business Portfolio has a WhatsApp Business Account."

        "no_phone_numbers" ->
            "No WhatsApp phone numbers were found on that WhatsApp Business Account."

        "phone_not_found" ->
            "Could not verify that WhatsApp phone number."

        "phone_in_use" ->
            "That WhatsApp phone number is already connected to another account."

        "token_exchange_failed" ->
            "Could not exchange Meta authorization code. Please try again."

        "meta_profile_failed" ->
            "Could not verify your Meta account. Please try again."

        "invalid_state" ->
            "Your signup session expired. Please try again."

        "missing_code" ->
            "WhatsApp signup was cancelled or did not return an authorization code."

        other ->
            "WhatsApp signup failed (" ++ other ++ ")."



-- UPDATE


type Msg
    = UrlChanged Url
    | LinkClicked Browser.UrlRequest
    | GotBootstrap (Api.Data Api.Bootstrap)
    | StartWhatsAppLogin
    | GotWhatsAppLogin (Api.Data Api.MetaSignup)
    | MetaLoginOutcome Ports.MetaSignupOutcome
    | GotPostLoginBootstrap (Api.Data Api.Bootstrap)
    | Logout
    | LoggedOut (Api.Data ())
    | LandingMsg Landing.Msg
    | WizardMsg Wizard.Msg
    | DashboardMsg Dashboard.Msg
    | NoOp


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
            let
                nextRoute =
                    Route.fromUrl url

                urlErr =
                    parseQueryError url.query

                modelWithErr =
                    case urlErr of
                        Just _ ->
                            { model | authError = urlErr }

                        Nothing ->
                            model
            in
            if nextRoute == Route.Landing && model.route == Route.Landing && model.session == Nothing then
                ( modelWithErr, scrollToFragment url.fragment )

            else
                let
                    ( nextModel, cmd ) =
                        enterRoute nextRoute modelWithErr
                in
                ( nextModel, Cmd.batch [ cmd, scrollToFragment url.fragment ] )

        GotBootstrap result ->
            let
                session =
                    case result of
                        Success boot ->
                            boot.session

                        _ ->
                            Nothing
            in
            enterRoute model.route { model | boot = result, session = session }

        StartWhatsAppLogin ->
            ( { model | authBusy = True, authError = Nothing }
            , Api.startWhatsAppLogin GotWhatsAppLogin
            )

        GotWhatsAppLogin result ->
            case result of
                Success signup ->
                    ( model
                    , Ports.openMetaSignup
                        { appId = signup.appId
                        , configId = signup.configId
                        , state = signup.state
                        }
                    )

                Failure err ->
                    ( { model | authBusy = False, authError = Just (Api.errorMessage err) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        MetaLoginOutcome outcome ->
            case outcome of
                Ports.MetaSignupOk ->
                    ( model, Api.getBootstrap GotPostLoginBootstrap )

                Ports.MetaSignupCancelled ->
                    ( { model | authBusy = False }, Cmd.none )

                Ports.MetaSignupError err ->
                    ( { model | authBusy = False, authError = Just err }, Cmd.none )

        GotPostLoginBootstrap result ->
            case result of
                Success boot ->
                    case boot.session of
                        Just session ->
                            let
                                dest =
                                    if session.destination == "dashboard" then
                                        Route.Dashboard

                                    else
                                        Route.Wizard

                                ( settled, cmd ) =
                                    enterRoute dest
                                        { model
                                            | boot = Success boot
                                            , session = Just session
                                            , authBusy = False
                                            , authError = Nothing
                                        }
                            in
                            ( settled, Cmd.batch [ Nav.pushUrl model.key (Route.toPath dest), cmd ] )

                        Nothing ->
                            ( { model
                                | boot = Success boot
                                , session = Nothing
                                , authBusy = False
                                , authError = Just "Sign-in did not complete. Please try again."
                              }
                            , Cmd.none
                            )

                Failure err ->
                    ( { model | authBusy = False, authError = Just (Api.errorMessage err) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        Logout ->
            let
                maybeDemo =
                    case model.boot of
                        Success boot ->
                            Just boot.demo

                        _ ->
                            Nothing
            in
            ( { model
                | session = Nothing
                , route = Route.Landing
                , page = LandingPage (Landing.init model.reducedMotion maybeDemo)
              }
            , Cmd.batch
                [ Api.logout LoggedOut
                , Nav.pushUrl model.key (Route.toPath Route.Landing)
                ]
            )

        LoggedOut _ ->
            ( model, Cmd.none )

        LandingMsg Landing.LoginClicked ->
            update StartWhatsAppLogin model

        LandingMsg sub ->
            case model.page of
                LandingPage landing ->
                    let
                        ( updated, cmd ) =
                            Landing.update sub landing
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

        NoOp ->
            ( model, Cmd.none )


scrollToFragment : Maybe String -> Cmd Msg
scrollToFragment maybeFrag =
    case maybeFrag of
        Just frag ->
            Dom.getElement frag
                |> Task.andThen (\info -> Dom.setViewport 0 info.element.y)
                |> Task.attempt (\_ -> NoOp)

        Nothing ->
            Cmd.none


markOnboarded : Api.Session -> Api.Session
markOnboarded session =
    { session | destination = "dashboard" }


signedOut : Model -> ( Model, Cmd Msg )
signedOut model =
    ( { model | session = Nothing, page = Stateless, route = Route.Login }
    , Nav.pushUrl model.key (Route.toPath Route.Login)
    )


enterRoute : Route -> Model -> ( Model, Cmd Msg )
enterRoute route model =
    let
        settled =
            { model | route = route }

        maybeDemo =
            case model.boot of
                Success boot ->
                    Just boot.demo

                _ ->
                    Nothing
    in
    case route of
        Route.Landing ->
            case model.session of
                Just session ->
                    redirectTo
                        (if session.destination == "wizard" then
                            Route.Wizard

                         else
                            Route.Dashboard
                        )
                        settled

                Nothing ->
                    ( { settled | page = LandingPage (Landing.init model.reducedMotion maybeDemo) }, Cmd.none )

        Route.Login ->
            case model.session of
                Just session ->
                    redirectTo
                        (if session.destination == "wizard" then
                            Route.Wizard

                         else
                            Route.Dashboard
                        )
                        settled

                Nothing ->
                    ( { settled | page = Stateless }, Cmd.none )

        Route.Demo ->
            ( { settled | page = LandingPage (Landing.init model.reducedMotion maybeDemo) }, Cmd.none )

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
                                Wizard.init model.authError
                        in
                        ( { settled | page = WizardPage wizard, authError = Nothing }, Cmd.map WizardMsg cmd )

        Route.Dashboard ->
            dashboardRoute Dashboard.Bots settled

        Route.DashboardBots ->
            dashboardRoute Dashboard.Bots settled

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
                    DashboardPage existing ->
                        let
                            ( updated, cmd ) =
                                Dashboard.switchTab tab existing
                        in
                        ( { model | page = DashboardPage updated }, Cmd.map DashboardMsg cmd )

                    _ ->
                        let
                            ( dash, cmd ) =
                                Dashboard.init tab model.authError
                        in
                        ( { model | page = DashboardPage dash, authError = Nothing }, Cmd.map DashboardMsg cmd )


needsSignIn : Model -> ( Model, Cmd Msg )
needsSignIn model =
    ( { model | page = Stateless, route = Route.Login }, Cmd.none )


redirectTo : Route -> Model -> ( Model, Cmd Msg )
redirectTo route model =
    ( model, Nav.replaceUrl model.key (Route.toPath route) )


subscriptions : Model -> Sub Msg
subscriptions model =
    let
        pageSub =
            case model.page of
                LandingPage landing ->
                    let
                        maybeDemo =
                            case model.boot of
                                Success boot ->
                                    Just boot.demo

                                _ ->
                                    Nothing
                    in
                    Sub.map LandingMsg (Landing.subscriptions maybeDemo landing)

                WizardPage _ ->
                    Sub.map WizardMsg Wizard.subscriptions

                DashboardPage _ ->
                    Sub.map DashboardMsg Dashboard.subscriptions

                _ ->
                    Sub.none

        authSub =
            if model.authBusy then
                Ports.metaSignupOutcome (Ports.decodeMetaSignupOutcome >> MetaLoginOutcome)

            else
                Sub.none
    in
    Sub.batch [ pageSub, authSub ]



-- VIEW


view : Model -> Browser.Document Msg
view model =
    { title = title model.route
    , body =
        [ Ui.header { session = model.session, onLogout = Logout }
        , main_ [ class "site-main" ] [ content model ]
        , Ui.footer
        ]
    }


title : Route -> String
title route =
    case route of
        Route.Landing ->
            "replybots — automatic WhatsApp replies for small businesses"

        Route.Demo ->
            "Live demo — replybots"

        Route.Terms ->
            "Terms of Service — replybots"

        Route.Privacy ->
            "Privacy Policy — replybots"

        Route.Login ->
            "Sign in — replybots"

        Route.Wizard ->
            "Set up replybots"

        Route.NotFound ->
            "Not found — replybots"

        _ ->
            "Dashboard — replybots"


content : Model -> Html Msg
content model =
    let
        loginHtml =
            Static.login
                { onLogin = StartWhatsAppLogin
                , busy = model.authBusy
                , error = model.authError
                }
    in
    case model.route of
        Route.Terms ->
            Static.terms

        Route.Privacy ->
            Static.privacy

        Route.Login ->
            loginHtml

        Route.NotFound ->
            Static.notFound

        Route.Demo ->
            case model.page of
                LandingPage landing ->
                    Html.map LandingMsg
                        (Landing.demoPage
                            model.authBusy
                            (RemoteData.map .demo model.boot)
                            landing
                        )

                _ ->
                    Ui.spinner

        Route.Landing ->
            case model.page of
                LandingPage landing ->
                    Html.map LandingMsg
                        (Landing.view
                            model.authBusy
                            (RemoteData.map .pricing model.boot)
                            (RemoteData.map .demo model.boot)
                            landing
                        )

                _ ->
                    Ui.spinner

        _ ->
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
                                    loginHtml

                        _ ->
                            Static.notFound
