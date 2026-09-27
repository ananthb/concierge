module Route exposing (Route(..), fromUrl, href, toPath)

{-| URL parsing. The list here must stay in step with `is_app_route` in
`src/lib.rs`: the worker only serves the SPA shell for paths it recognises,
so a route added here without a matching arm there 404s on a hard refresh
(it would still work when reached by in-app navigation, which is the
confusing part).
-}

import Html
import Html.Attributes
import Url exposing (Url)
import Url.Parser as Parser exposing ((</>), Parser, oneOf, s, top)


type Route
    = Landing
    | Pricing
    | Features
    | Terms
    | Privacy
    | Login
    | Wizard
    | Dashboard
      -- Sub-pages of the signed-in app. Kept as distinct routes rather than
      -- tab state so they're linkable and the back button works.
    | DashboardPersona
    | DashboardChannels
    | DashboardBilling
    | DashboardSettings
    | Manage
    | NotFound


parser : Parser (Route -> a) a
parser =
    oneOf
        [ Parser.map Landing top
        , Parser.map Pricing (s "pricing")
        , Parser.map Features (s "features")
        , Parser.map Terms (s "terms")
        , Parser.map Privacy (s "privacy")
        , Parser.map Login (s "login")
        , Parser.map Wizard (s "wizard")
        , Parser.map DashboardPersona (s "dashboard" </> s "persona")
        , Parser.map DashboardChannels (s "dashboard" </> s "channels")
        , Parser.map DashboardBilling (s "dashboard" </> s "billing")
        , Parser.map DashboardSettings (s "dashboard" </> s "settings")
        , Parser.map Dashboard (s "dashboard")
        , Parser.map Manage (s "manage")
        ]


fromUrl : Url -> Route
fromUrl url =
    Parser.parse parser url |> Maybe.withDefault NotFound


toPath : Route -> String
toPath route =
    case route of
        Landing ->
            "/"

        Pricing ->
            "/pricing"

        Features ->
            "/features"

        Terms ->
            "/terms"

        Privacy ->
            "/privacy"

        Login ->
            "/login"

        Wizard ->
            "/wizard"

        Dashboard ->
            "/dashboard"

        DashboardPersona ->
            "/dashboard/persona"

        DashboardChannels ->
            "/dashboard/channels"

        DashboardBilling ->
            "/dashboard/billing"

        DashboardSettings ->
            "/dashboard/settings"

        Manage ->
            "/manage"

        NotFound ->
            "/"


href : Route -> Html.Attribute msg
href route =
    Html.Attributes.href (toPath route)
