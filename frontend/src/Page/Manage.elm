module Page.Manage exposing (Model, Msg, init, update, view)

{-| The operator console at `/manage`.

Everything here talks to `/api/manage/*`, which is gated by Cloudflare Access
rather than by the session cookie. That has two consequences worth stating:

  - **There is no sign-in here.** The `CF_Authorization` cookie travels with
    the request like any other; if Access hasn't issued one, every call comes
    back `access_required`. The console renders that as an explanation of what
    is missing rather than as a red failure banner, because it isn't an error
    so much as a door the visitor hasn't come through.

  - **The page itself is not the security boundary.** Access should cover
    `/manage*` as well as `/api/manage*`. If it only covers the API, this page
    loads for anyone and every panel on it says the same thing.

Tabs are local state rather than routes. The dashboard puts each tab in the
URL because tenants link each other to "the billing page"; nobody deep-links
an operator console, and a route per tab would mean five more entries in
`Route` for no one's benefit.

-}

import Api
import Format
import Html exposing (Html, div, h1, h2, h3, label, li, p, section, span, table, tbody, td, text, th, thead, tr, ul)
import Html.Attributes exposing (class, classList)
import Html.Events exposing (onClick)
import RemoteData exposing (RemoteData(..))
import Ui



-- MODEL


type Tab
    = Overview
    | Pricing
    | Demo
    | Archetypes
    | Tenants
    | Audit


type alias Model =
    { tab : Tab
    , overview : Api.Data Api.Overview
    , pricing : Api.Data Api.AdminPricing
    , demo : Api.Data Api.AdminDemo
    , archetypes : Api.Data (List Api.AdminArchetype)
    , tenants : Api.Data (List Api.TenantRow)
    , audit : Api.Data (List Api.AuditEntry)

    -- Editable copies. The `Data` above stays as the server last answered,
    -- so a failed save leaves the form's text alone rather than reverting
    -- what the operator typed.
    , pricingForm : PricingForm
    , demoForm : Maybe Api.AdminDemo
    , archetypeForm : Maybe ArchetypeForm
    , archetypeDelete : Maybe String
    , tenantQuery : String
    , selected : Api.Data Api.TenantDetail
    , grantCount : String
    , grantExpiryDays : String
    , deleteConfirm : String
    , auditActor : String
    , auditAction : String
    , saving : Bool
    , notice : Maybe ( String, String )
    }


{-| Pricing edits as text, keyed by `(concept, currency)`. Amounts are
integers on the wire — minor units, or thousandths of one for the `is_milli`
concepts — so the box holds exactly what will be sent and the human reading
of it is rendered beside the field.
-}
type alias PricingForm =
    { minCredits : String
    , maxCredits : String
    , amounts : List AmountField
    }


{-| One editable cell. The text is held as typed, not as a parsed int:
binding the box to a parsed value means deleting the last digit snaps it
back, so you can never clear a field to retype it.
-}
type alias AmountField =
    { concept : String
    , currency : String
    , text : String
    }


{-| The catalog editor's text. Chip lists are newline-delimited while
editing, the same as the persona form: a textarea is the right control for
"one per line", and converting on save keeps every keystroke from
restructuring a list.
-}
type alias ArchetypeForm =
    { slug : String
    , label : String
    , description : String
    , voicePrompt : String
    , greeting : String
    , catchPhrases : String
    , offTopics : String
    , never : String
    , handoffConditions : String
    , isNew : Bool
    }


emptyArchetypeForm : ArchetypeForm
emptyArchetypeForm =
    { slug = ""
    , label = ""
    , description = ""
    , voicePrompt = ""
    , greeting = ""
    , catchPhrases = ""
    , offTopics = ""
    , never = ""
    , handoffConditions = ""
    , isNew = True
    }


formFromArchetype : Api.AdminArchetype -> ArchetypeForm
formFromArchetype row =
    { slug = row.slug
    , label = row.label
    , description = row.description
    , voicePrompt = row.voicePrompt
    , greeting = row.greeting
    , catchPhrases = String.join "\n" row.catchPhrases
    , offTopics = String.join "\n" row.offTopics
    , never = row.never
    , handoffConditions = String.join "\n" row.handoffConditions
    , isNew = False
    }


init : ( Model, Cmd Msg )
init =
    ( { tab = Overview
      , overview = Loading
      , pricing = NotAsked
      , demo = NotAsked
      , archetypes = NotAsked
      , tenants = NotAsked
      , audit = NotAsked
      , pricingForm = { minCredits = "", maxCredits = "", amounts = [] }
      , demoForm = Nothing
      , archetypeForm = Nothing
      , archetypeDelete = Nothing
      , tenantQuery = ""
      , selected = NotAsked
      , grantCount = ""
      , grantExpiryDays = ""
      , deleteConfirm = ""
      , auditActor = ""
      , auditAction = ""
      , saving = False
      , notice = Nothing
      }
    , Api.getOverview GotOverview
    )



-- UPDATE


type Msg
    = SwitchTo Tab
    | GotOverview (Api.Data Api.Overview)
    | GotPricing (Api.Data Api.AdminPricing)
    | SetMinCredits String
    | SetMaxCredits String
    | SetAmount AmountField String
    | SavePricing
    | GotDemo (Api.Data Api.AdminDemo)
    | EditDemo (Api.AdminDemo -> Api.AdminDemo)
    | SaveDemo
    | RerollDemo
    | Rerolled (Api.Data ())
    | GotArchetypes (Api.Data (List Api.AdminArchetype))
    | NewArchetype
    | OpenArchetype Api.AdminArchetype
    | CloseArchetype
    | EditArchetype (ArchetypeForm -> ArchetypeForm)
    | SaveArchetype
    | SavedArchetype (Api.Data Api.AdminArchetype)
    | AskDeleteArchetype String
    | CancelDeleteArchetype
    | DeleteArchetype String
    | DeletedArchetype (Api.Data ())
    | SetTenantQuery String
    | SearchTenants
    | GotTenants (Api.Data (List Api.TenantRow))
    | OpenTenant String
    | GotTenant (Api.Data Api.TenantDetail)
    | CloseTenant
    | SetGrantCount String
    | SetGrantExpiry String
    | GrantReplies
    | Granted (Api.Data ())
    | SetDeleteConfirm String
    | DeleteTenant
    | Deleted (Api.Data ())
    | SetAuditActor String
    | SetAuditAction String
    | SearchAudit
    | GotAudit (Api.Data (List Api.AuditEntry))


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SwitchTo tab ->
            ( { model | tab = tab, notice = Nothing }, fetchFor tab model )

        GotOverview result ->
            ( { model | overview = result }, Cmd.none )

        GotPricing result ->
            ( { model
                | pricing = result
                , saving = False
                , pricingForm =
                    case result of
                        Success pricing ->
                            { minCredits = String.fromInt pricing.minCredits
                            , maxCredits = String.fromInt pricing.maxCredits
                            , amounts =
                                List.map
                                    (\a ->
                                        { concept = a.concept
                                        , currency = a.currency
                                        , text = String.fromInt a.amount
                                        }
                                    )
                                    pricing.amounts
                            }

                        _ ->
                            model.pricingForm
                , notice =
                    case ( result, model.saving ) of
                        ( Success _, True ) ->
                            Just ( "success", "Pricing saved." )

                        ( Failure err, _ ) ->
                            Just ( "error", Api.errorMessage err )

                        _ ->
                            model.notice
              }
            , Cmd.none
            )

        SetMinCredits value ->
            ( { model | pricingForm = setMin value model.pricingForm }, Cmd.none )

        SetMaxCredits value ->
            ( { model | pricingForm = setMax value model.pricingForm }, Cmd.none )

        SetAmount target value ->
            let
                form =
                    model.pricingForm

                updated =
                    List.map
                        (\a ->
                            if a.concept == target.concept && a.currency == target.currency then
                                { a | text = value }

                            else
                                a
                        )
                        form.amounts
            in
            ( { model | pricingForm = { form | amounts = updated } }, Cmd.none )

        SavePricing ->
            let
                form =
                    model.pricingForm
            in
            case ( String.toInt form.minCredits, String.toInt form.maxCredits, parseAmounts form.amounts ) of
                ( Just minCredits, Just maxCredits, Ok amounts ) ->
                    ( { model | saving = True, notice = Nothing }
                    , Api.saveAdminPricing
                        { minCredits = minCredits
                        , maxCredits = maxCredits
                        , amounts = amounts
                        }
                        GotPricing
                    )

                ( _, _, Err badCell ) ->
                    ( { model | notice = Just ( "error", badCell ) }, Cmd.none )

                _ ->
                    ( { model
                        | notice = Just ( "error", "Credit bounds have to be whole numbers." )
                      }
                    , Cmd.none
                    )

        GotDemo result ->
            ( { model
                | demo = result
                , saving = False
                , demoForm =
                    case result of
                        Success demo ->
                            Just demo

                        _ ->
                            model.demoForm
                , notice =
                    case ( result, model.saving ) of
                        ( Success _, True ) ->
                            Just ( "success", "Demo settings saved." )

                        ( Failure err, _ ) ->
                            Just ( "error", Api.errorMessage err )

                        _ ->
                            model.notice
              }
            , Cmd.none
            )

        EditDemo change ->
            ( { model | demoForm = Maybe.map change model.demoForm }, Cmd.none )

        SaveDemo ->
            case model.demoForm of
                Just demo ->
                    ( { model | saving = True, notice = Nothing }
                    , Api.saveAdminDemo demo GotDemo
                    )

                Nothing ->
                    ( model, Cmd.none )

        RerollDemo ->
            ( { model | saving = True, notice = Nothing }, Api.rerollDemo Rerolled )

        Rerolled result ->
            case result of
                Success () ->
                    -- Re-read rather than trusting the local copy: the roll
                    -- stamps a new `generated_at`, which is the only way to
                    -- tell it actually happened.
                    ( { model | saving = False, notice = Just ( "success", "Rerolled the demo personas." ) }
                    , Api.getAdminDemo GotDemo
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        GotArchetypes result ->
            ( { model | archetypes = result }, Cmd.none )

        NewArchetype ->
            ( { model
                | archetypeForm = Just emptyArchetypeForm
                , archetypeDelete = Nothing
                , notice = Nothing
              }
            , Cmd.none
            )

        OpenArchetype row ->
            ( { model
                | archetypeForm = Just (formFromArchetype row)
                , archetypeDelete = Nothing
                , notice = Nothing
              }
            , Cmd.none
            )

        CloseArchetype ->
            ( { model | archetypeForm = Nothing }, Cmd.none )

        EditArchetype change ->
            ( { model | archetypeForm = Maybe.map change model.archetypeForm }, Cmd.none )

        SaveArchetype ->
            case Maybe.map validateArchetype model.archetypeForm of
                Just (Ok row) ->
                    ( { model | saving = True, notice = Nothing }
                    , Api.saveAdminArchetype row SavedArchetype
                    )

                Just (Err message) ->
                    ( { model | notice = Just ( "error", message ) }, Cmd.none )

                Nothing ->
                    ( model, Cmd.none )

        SavedArchetype result ->
            case result of
                Success row ->
                    ( { model
                        | saving = False
                        , archetypeForm = Nothing
                        , archetypeDelete = Nothing
                        , archetypes = Loading
                        , notice =
                            Just
                                ( "success"
                                , "Saved " ++ row.slug ++ ". Its safety check is queued; if the voice prompt changed, tenants using it stay paused until the check approves it and they re-save their persona."
                                )
                      }
                    , Api.getAdminArchetypes GotArchetypes
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        AskDeleteArchetype slug ->
            ( { model | archetypeDelete = Just slug, notice = Nothing }, Cmd.none )

        CancelDeleteArchetype ->
            ( { model | archetypeDelete = Nothing }, Cmd.none )

        DeleteArchetype slug ->
            ( { model | saving = True, notice = Nothing }
            , Api.deleteAdminArchetype slug DeletedArchetype
            )

        DeletedArchetype result ->
            case result of
                Success () ->
                    ( { model
                        | saving = False
                        , archetypeDelete = Nothing
                        , archetypes = Loading
                        , notice = Just ( "success", "Deleted the voice." )
                      }
                    , Api.getAdminArchetypes GotArchetypes
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        SetTenantQuery value ->
            ( { model | tenantQuery = value }, Cmd.none )

        SearchTenants ->
            ( { model | tenants = Loading }, Api.listTenants model.tenantQuery GotTenants )

        GotTenants result ->
            ( { model | tenants = result }, Cmd.none )

        OpenTenant id ->
            ( { model
                | selected = Loading
                , grantCount = ""
                , grantExpiryDays = ""
                , deleteConfirm = ""
                , notice = Nothing
              }
            , Api.getTenantDetail id GotTenant
            )

        GotTenant result ->
            ( { model | selected = result }, Cmd.none )

        CloseTenant ->
            ( { model | selected = NotAsked, deleteConfirm = "" }, Cmd.none )

        SetGrantCount value ->
            ( { model | grantCount = value }, Cmd.none )

        SetGrantExpiry value ->
            ( { model | grantExpiryDays = value }, Cmd.none )

        GrantReplies ->
            case ( model.selected, String.toInt (String.trim model.grantCount) ) of
                ( Success detail, Just count ) ->
                    ( { model | saving = True, notice = Nothing }
                    , Api.grantReplies detail.tenant.id
                        { count = count
                        , expiresDays = String.toInt (String.trim model.grantExpiryDays)
                        }
                        Granted
                    )

                ( Success _, Nothing ) ->
                    ( { model | notice = Just ( "error", "How many replies?" ) }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        Granted result ->
            case ( result, model.selected ) of
                ( Success (), Success detail ) ->
                    ( { model
                        | saving = False
                        , grantCount = ""
                        , grantExpiryDays = ""
                        , notice = Just ( "success", "Granted." )
                      }
                    , Api.getTenantDetail detail.tenant.id GotTenant
                    )

                ( Failure err, _ ) ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        SetDeleteConfirm value ->
            ( { model | deleteConfirm = value }, Cmd.none )

        DeleteTenant ->
            case model.selected of
                Success detail ->
                    ( { model | saving = True, notice = Nothing }
                    , Api.deleteTenant detail.tenant.id Deleted
                    )

                _ ->
                    ( model, Cmd.none )

        Deleted result ->
            case result of
                Success () ->
                    ( { model
                        | saving = False
                        , selected = NotAsked
                        , deleteConfirm = ""
                        , tenants = Loading
                        , notice = Just ( "success", "Tenant deleted." )
                      }
                    , Api.listTenants model.tenantQuery GotTenants
                    )

                Failure err ->
                    ( { model | saving = False, notice = Just ( "error", Api.errorMessage err ) }
                    , Cmd.none
                    )

                _ ->
                    ( model, Cmd.none )

        SetAuditActor value ->
            ( { model | auditActor = value }, Cmd.none )

        SetAuditAction value ->
            ( { model | auditAction = value }, Cmd.none )

        SearchAudit ->
            ( { model | audit = Loading }
            , Api.getAudit { actor = model.auditActor, action = model.auditAction } GotAudit
            )

        GotAudit result ->
            ( { model | audit = result }, Cmd.none )


{-| Load a tab's data the first time it's opened. Re-visiting keeps what's in
hand, same as the dashboard.
-}
fetchFor : Tab -> Model -> Cmd Msg
fetchFor tab model =
    case tab of
        Overview ->
            if RemoteData.isNotAsked model.overview then
                Api.getOverview GotOverview

            else
                Cmd.none

        Pricing ->
            if RemoteData.isNotAsked model.pricing then
                Api.getAdminPricing GotPricing

            else
                Cmd.none

        Demo ->
            if RemoteData.isNotAsked model.demo then
                Api.getAdminDemo GotDemo

            else
                Cmd.none

        Archetypes ->
            if RemoteData.isNotAsked model.archetypes then
                Api.getAdminArchetypes GotArchetypes

            else
                Cmd.none

        Tenants ->
            if RemoteData.isNotAsked model.tenants then
                Api.listTenants "" GotTenants

            else
                Cmd.none

        Audit ->
            if RemoteData.isNotAsked model.audit then
                Api.getAudit { actor = "", action = "" } GotAudit

            else
                Cmd.none



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "dashboard manage" ]
        [ h1 [] [ text "Operations" ]
        , tabs model.tab
        , case model.notice of
            Just ( kind, message ) ->
                Ui.banner kind message

            Nothing ->
                text ""
        , case model.tab of
            Overview ->
                overviewTab model

            Pricing ->
                pricingTab model

            Demo ->
                demoTab model

            Archetypes ->
                archetypesTab model

            Tenants ->
                tenantsTab model

            Audit ->
                auditTab model
        ]


tabs : Tab -> Html Msg
tabs current =
    ul [ class "tabs" ]
        (List.map
            (\( tab, label ) ->
                li []
                    [ Html.button
                        [ classList [ ( "tab", True ), ( "is-active", tab == current ) ]
                        , Html.Attributes.type_ "button"
                        , onClick (SwitchTo tab)
                        ]
                        [ text label ]
                    ]
            )
            [ ( Overview, "Overview" )
            , ( Pricing, "Pricing" )
            , ( Demo, "Demo" )
            , ( Archetypes, "Archetypes" )
            , ( Tenants, "Tenants" )
            , ( Audit, "Audit" )
            ]
        )


{-| Parse every cell, naming the first bad one rather than dropping it.
A silently skipped row would save the rest and leave that rate at whatever
the server already had, which looks like a successful save of a value the
operator never typed.
-}
parseAmounts : List AmountField -> Result String (List Api.PricingAmount)
parseAmounts fields =
    List.foldr
        (\field acc ->
            case ( acc, String.toInt (String.trim field.text) ) of
                ( Err e, _ ) ->
                    Err e

                ( Ok rest, Just amount ) ->
                    Ok ({ concept = field.concept, currency = field.currency, amount = amount } :: rest)

                ( Ok _, Nothing ) ->
                    Err (field.concept ++ " in " ++ field.currency ++ " has to be a whole number.")
        )
        (Ok [])
        fields


setMin : String -> PricingForm -> PricingForm
setMin value form =
    { form | minCredits = value }


setMax : String -> PricingForm -> PricingForm
setMax value form =
    { form | maxCredits = value }


{-| Render a panel, turning the one failure that isn't really a failure into
an explanation. Access denial is the expected state for anyone who hasn't
come through the operator door.
-}
panel : Api.Data a -> (a -> Html Msg) -> Html Msg
panel data toHtml =
    case data of
        Failure (Api.Fault fault) ->
            if fault.code == "access_required" then
                Ui.card
                    [ h2 [] [ text "This console needs Cloudflare Access" ]
                    , p []
                        [ text "The operator endpoints authenticate with an Access JWT, not a Concierge session. This page loaded, which means Access isn't in front of it — if you're seeing this as an operator, the Access application needs to cover both /manage and /api/manage." ]
                    , p [ class "muted" ] [ text fault.message ]
                    ]

            else
                Ui.banner "error" fault.message

        _ ->
            Ui.remote data toHtml


overviewTab : Model -> Html Msg
overviewTab model =
    panel model.overview <|
        \overview ->
            section []
                [ Ui.card
                    [ h2 [] [ text "Signed in as" ]
                    , p [ class "mono" ] [ text overview.actor ]
                    , p [ class "muted" ]
                        [ text "This address is what the audit log records for anything you do here." ]
                    ]
                , Ui.card
                    [ h2 [] [ text "Accounts" ]
                    , p [ class "balance" ] [ text (Format.count "en-IN" overview.tenantCount) ]
                    ]
                , Ui.card
                    [ h2 [] [ text "Health" ]
                    , p []
                        [ span
                            [ classList
                                [ ( "pill", True )
                                , ( "pill-ok", overview.health.overall == "ok" )
                                , ( "pill-warn", overview.health.overall /= "ok" )
                                ]
                            ]
                            [ text overview.health.overall ]
                        ]
                    , ul [ class "connected-list" ]
                        (List.map
                            (\check ->
                                li [ class "connected-item" ]
                                    [ span [ class "connected-name" ] [ text check.name ]
                                    , span [ class "muted" ] [ text check.detail ]
                                    , span
                                        [ classList
                                            [ ( "pill", True )
                                            , ( "pill-ok", check.status == "ok" )
                                            , ( "pill-warn", check.status /= "ok" )
                                            ]
                                        ]
                                        [ text check.status ]
                                    ]
                            )
                            overview.health.checks
                        )
                    ]
                ]


pricingTab : Model -> Html Msg
pricingTab model =
    panel model.pricing <|
        \pricing ->
            section []
                [ Ui.card
                    [ h2 [] [ text "What a reply costs" ]
                    , p [ class "muted" ]
                        [ text "Amounts are whole numbers in the currency's smallest unit, or thousandths of it where the concept says so. The reading underneath each box is what a customer sees." ]
                    , table [ class "rate-table" ]
                        [ thead []
                            [ tr []
                                [ th [] [ text "Concept" ]
                                , th [] [ text "Currency" ]
                                , th [] [ text "Amount" ]
                                ]
                            ]
                        , tbody []
                            (List.map (amountRow pricing) model.pricingForm.amounts)
                        ]
                    ]
                , Ui.card
                    [ h2 [] [ text "How much they can buy at once" ]
                    , Ui.field
                        { id = "min_credits"
                        , label = "Minimum replies per purchase"
                        , value = model.pricingForm.minCredits
                        , hint = ""
                        , required = False
                        , onInput = SetMinCredits
                        }
                    , Ui.field
                        { id = "max_credits"
                        , label = "Maximum replies per purchase"
                        , value = model.pricingForm.maxCredits
                        , hint =
                            "Hard ceiling: "
                                ++ Format.count "en-IN" pricing.maxCreditsCeiling
                                ++ "."
                        , required = False
                        , onInput = SetMaxCredits
                        }
                    , div [ class "card-actions" ]
                        [ Ui.button
                            { label = "Save pricing"
                            , onClick = SavePricing
                            , primary = True
                            , busy = model.saving
                            }
                        ]
                    ]
                ]


amountRow : Api.AdminPricing -> AmountField -> Html Msg
amountRow pricing amount =
    let
        concept =
            List.filter (\c -> c.wire == amount.concept) pricing.concepts
                |> List.head

        label =
            Maybe.map .label concept |> Maybe.withDefault amount.concept

        caption =
            Maybe.map .unitCaption concept |> Maybe.withDefault ""

        reading =
            case ( String.toInt (String.trim amount.text), Maybe.map .isMilli concept ) of
                ( Nothing, _ ) ->
                    "—"

                ( Just value, Just True ) ->
                    Format.milliRate "en-IN" amount.currency value

                ( Just value, _ ) ->
                    Format.money "en-IN" amount.currency value
    in
    tr []
        [ td []
            [ text label
            , p [ class "hint" ] [ text caption ]
            ]
        , td [ class "mono" ] [ text amount.currency ]
        , td []
            [ Html.input
                [ Html.Attributes.type_ "text"
                , Html.Attributes.value amount.text
                , Html.Attributes.id (amount.concept ++ "-" ++ amount.currency)
                , Html.Events.onInput (SetAmount amount)
                ]
                []
            , p [ class "hint" ] [ text reading ]
            ]
        ]


demoTab : Model -> Html Msg
demoTab model =
    panel model.demo <|
        \_ ->
            case model.demoForm of
                Nothing ->
                    text ""

                Just demo ->
                    section []
                        [ Ui.card
                            [ h2 [] [ text "The landing page demo" ]
                            , Ui.toggle
                                { id = "demo_enabled"
                                , label = "Show the demo to visitors"
                                , checked = demo.enabled
                                , onCheck = \v -> EditDemo (\d -> { d | enabled = v })
                                }
                            , p [ class "muted" ]
                                [ text
                                    (case demo.generatedAt of
                                        Just at ->
                                            "Personas last rolled " ++ String.left 16 at ++ "."

                                        Nothing ->
                                            "No personas stored — the next visitor rolls a set."
                                    )
                                ]
                            , div [ class "card-actions" ]
                                [ Ui.button
                                    { label = "Reroll personas now"
                                    , onClick = RerollDemo
                                    , primary = False
                                    , busy = model.saving
                                    }
                                ]
                            ]
                        , Ui.card
                            [ h2 [] [ text "Limits" ]
                            , numberField "demo_turns"
                                "Messages a visitor gets"
                                "The server enforces this too; the page just stops asking."
                                demo.maxUserTurns
                                (\n d -> { d | maxUserTurns = n })
                            , numberField "demo_idle"
                                "Seconds of silence before the sign-up prompt"
                                "Restarts on every keystroke."
                                demo.idleTimeoutSecs
                                (\n d -> { d | idleTimeoutSecs = n })
                            , numberField "demo_cadence"
                                "Minutes between automatic rerolls"
                                "Zero turns the cron reroll off; the button above still works."
                                demo.regenerationCadenceMins
                                (\n d -> { d | regenerationCadenceMins = n })
                            ]
                        , Ui.card
                            [ h2 [] [ text "How the personas are written" ]
                            , Ui.textarea
                                { id = "demo_prompt"
                                , label = "Generation prompt"
                                , value = demo.personaGenerationPrompt
                                , hint = "Saving a different prompt discards the stored personas, so the next visitor rolls against the new one."
                                , rows = 10
                                , onInput = \v -> EditDemo (\d -> { d | personaGenerationPrompt = v })
                                }
                            , div [ class "card-actions" ]
                                [ Ui.button
                                    { label = "Save demo settings"
                                    , onClick = SaveDemo
                                    , primary = True
                                    , busy = model.saving
                                    }
                                , Ui.button
                                    { label = "Reset the prompt"
                                    , onClick =
                                        EditDemo (\d -> { d | personaGenerationPrompt = demo.defaultPrompt })
                                    , primary = False
                                    , busy = False
                                    }
                                ]
                            ]
                        ]


numberField : String -> String -> String -> Int -> (Int -> Api.AdminDemo -> Api.AdminDemo) -> Html Msg
numberField id label hint value change =
    Ui.field
        { id = id
        , label = label
        , value = String.fromInt value
        , hint = hint
        , required = False
        , onInput =
            \v ->
                EditDemo
                    (\d ->
                        -- A half-typed box shouldn't wipe the stored number:
                        -- an unparseable value keeps the current one.
                        change (String.toInt v |> Maybe.withDefault value) d
                    )
        }


archetypesTab : Model -> Html Msg
archetypesTab model =
    panel model.archetypes <|
        \rows ->
            section []
                [ Ui.card
                    [ h2 [] [ text "Voice catalog" ]
                    , p [ class "muted" ]
                        [ text "The voices the tenant persona picker and the landing-page demo read from. Every write re-runs the safety check before the row can be used again." ]
                    , div [ class "card-actions" ]
                        [ Ui.button
                            { label = "Add a voice"
                            , onClick = NewArchetype
                            , primary = True
                            , busy = False
                            }
                        ]
                    ]
                , case model.archetypeForm of
                    Just form ->
                        archetypeFormCard model form

                    Nothing ->
                        text ""
                , case model.archetypeDelete of
                    Just slug ->
                        Ui.card
                            [ h2 [] [ text ("Delete " ++ slug ++ "?") ]
                            , p []
                                [ text "Removes it from the tenant picker and the demo. Tenants already using it lose its voice prompt, and their AI replies stay paused until they choose another voice and re-save." ]
                            , div [ class "card-actions" ]
                                [ Html.button
                                    [ class "btn btn-danger"
                                    , Html.Attributes.type_ "button"
                                    , Html.Attributes.disabled model.saving
                                    , onClick (DeleteArchetype slug)
                                    ]
                                    [ text "Delete permanently" ]
                                , Ui.button
                                    { label = "Cancel"
                                    , onClick = CancelDeleteArchetype
                                    , primary = False
                                    , busy = False
                                    }
                                ]
                            ]

                    Nothing ->
                        text ""
                , archetypeList rows
                ]


archetypeList : List Api.AdminArchetype -> Html Msg
archetypeList rows =
    if List.isEmpty rows then
        Ui.card [ p [ class "muted" ] [ text "No voices in the catalog yet." ] ]

    else
        Ui.card
            [ h2 [] [ text "In the catalog" ]
            , table [ class "rate-table" ]
                [ thead []
                    [ tr []
                        [ th [] [ text "Voice" ]
                        , th [] [ text "Slug" ]
                        , th [] [ text "Safety" ]
                        , th [] [ text "Updated" ]
                        , th [] [ text "" ]
                        ]
                    ]
                , tbody []
                    (List.map archetypeRow (List.sortBy .label rows))
                ]
            ]


archetypeRow : Api.AdminArchetype -> Html Msg
archetypeRow row =
    tr []
        [ td []
            [ text row.label
            , p [ class "hint" ] [ text row.description ]
            ]
        , td [ class "mono" ] [ text row.slug ]
        , td []
            [ safetyPill row.safetyStatus
            , case row.safetyReason of
                Just reason ->
                    p [ class "hint" ] [ text reason ]

                Nothing ->
                    text ""
            ]
        , td [ class "mono" ]
            [ text (Maybe.withDefault "—" (Maybe.map (String.left 10) row.updatedAt)) ]
        , td []
            [ div [ class "card-actions" ]
                [ Html.button
                    [ class "btn btn-ghost"
                    , Html.Attributes.type_ "button"
                    , onClick (OpenArchetype row)
                    ]
                    [ text "Edit" ]
                , Html.button
                    [ class "btn btn-ghost"
                    , Html.Attributes.type_ "button"
                    , onClick (AskDeleteArchetype row.slug)
                    ]
                    [ text "Delete" ]
                ]
            ]
        ]


safetyPill : String -> Html Msg
safetyPill status =
    span
        [ classList
            [ ( "pill", True )
            , ( "pill-ok", status == "approved" )
            , ( "pill-warn", status /= "approved" )
            ]
        ]
        [ text status ]


archetypeFormCard : Model -> ArchetypeForm -> Html Msg
archetypeFormCard model form =
    Ui.card
        [ h2 []
            [ text
                (if form.isNew then
                    "New voice"

                 else
                    "Edit " ++ form.slug
                )
            ]
        , if form.isNew then
            Ui.field
                { id = "archetype_slug"
                , label = "Slug"
                , value = form.slug
                , hint = "Lowercase letters, digits and hyphens. It is the key, so it cannot be changed later."
                , required = True
                , onInput = \v -> EditArchetype (\f -> { f | slug = v })
                }

          else
            div [ class "field" ]
                [ label [] [ text "Slug" ]
                , p [ class "mono" ] [ text form.slug ]
                , p [ class "hint" ] [ text "The slug is the key; create a new row to change it." ]
                ]
        , Ui.field
            { id = "archetype_label"
            , label = "Label"
            , value = form.label
            , hint = "Shown in the tenant picker."
            , required = True
            , onInput = \v -> EditArchetype (\f -> { f | label = v })
            }
        , Ui.field
            { id = "archetype_description"
            , label = "Description"
            , value = form.description
            , hint = "One sentence, shown under the label."
            , required = True
            , onInput = \v -> EditArchetype (\f -> { f | description = v })
            }
        , Ui.textarea
            { id = "archetype_voice"
            , label = "Voice prompt"
            , value = form.voicePrompt
            , hint = "The editable middle the persona builder layers the tenant's own details over."
            , rows = 10
            , onInput = \v -> EditArchetype (\f -> { f | voicePrompt = v })
            }
        , Ui.textarea
            { id = "archetype_greeting"
            , label = "Greeting"
            , value = form.greeting
            , hint = "The first message in the demo conversation."
            , rows = 3
            , onInput = \v -> EditArchetype (\f -> { f | greeting = v })
            }
        , Ui.textarea
            { id = "archetype_catch"
            , label = "Catch-phrases"
            , value = form.catchPhrases
            , hint = "One per line."
            , rows = 4
            , onInput = \v -> EditArchetype (\f -> { f | catchPhrases = v })
            }
        , Ui.textarea
            { id = "archetype_off"
            , label = "Off-topics"
            , value = form.offTopics
            , hint = "One per line. The persona declines these."
            , rows = 4
            , onInput = \v -> EditArchetype (\f -> { f | offTopics = v })
            }
        , Ui.field
            { id = "archetype_never"
            , label = "Never"
            , value = form.never
            , hint = "Optional line the prompt must never cross."
            , required = False
            , onInput = \v -> EditArchetype (\f -> { f | never = v })
            }
        , Ui.textarea
            { id = "archetype_handoff"
            , label = "Handoff conditions"
            , value = form.handoffConditions
            , hint = "One per line. When one matches, the conversation goes to a human."
            , rows = 4
            , onInput = \v -> EditArchetype (\f -> { f | handoffConditions = v })
            }
        , p [ class "hint" ]
            [ text "Saving resets the safety verdict to pending. If the voice prompt changes, tenants using this row stay paused until the check approves it and they re-save their persona." ]
        , div [ class "card-actions" ]
            [ Ui.button
                { label = "Save voice"
                , onClick = SaveArchetype
                , primary = True
                , busy = model.saving
                }
            , Ui.button
                { label = "Cancel"
                , onClick = CloseArchetype
                , primary = False
                , busy = False
                }
            ]
        ]


validateArchetype : ArchetypeForm -> Result String Api.AdminArchetype
validateArchetype form =
    let
        slug =
            String.toLower (String.trim form.slug)
    in
    if String.isEmpty slug then
        Err "Slug is required."

    else if not (String.all (\c -> Char.isLower c || Char.isDigit c || c == '-') slug) then
        Err "Slug can only contain lowercase letters, digits and hyphens."

    else
        case missingRequired [ ( "Label", form.label ), ( "Description", form.description ), ( "Voice prompt", form.voicePrompt ), ( "Greeting", form.greeting ) ] of
            Just name ->
                Err (name ++ " is required.")

            Nothing ->
                Ok
                    { slug = slug
                    , label = String.trim form.label
                    , description = String.trim form.description
                    , voicePrompt = String.trim form.voicePrompt
                    , greeting = String.trim form.greeting
                    , catchPhrases = textToChips form.catchPhrases
                    , offTopics = textToChips form.offTopics
                    , never = String.trim form.never
                    , handoffConditions = textToChips form.handoffConditions
                    , safetyStatus = "pending"
                    , safetyReason = Nothing
                    , updatedAt = Nothing
                    }


missingRequired : List ( String, String ) -> Maybe String
missingRequired fields =
    fields
        |> List.filter (\( _, value ) -> String.trim value == "")
        |> List.head
        |> Maybe.map Tuple.first


textToChips : String -> List String
textToChips =
    String.lines >> List.map String.trim >> List.filter (not << String.isEmpty)


tenantsTab : Model -> Html Msg
tenantsTab model =
    section []
        [ Ui.card
            [ h2 [] [ text "Find an account" ]
            , Ui.field
                { id = "tenant_q"
                , label = "Email or name"
                , value = model.tenantQuery
                , hint = "Empty lists everyone."
                , required = False
                , onInput = SetTenantQuery
                }
            , div [ class "card-actions" ]
                [ Ui.button
                    { label = "Search"
                    , onClick = SearchTenants
                    , primary = True
                    , busy = False
                    }
                ]
            ]
        , case model.selected of
            NotAsked ->
                panel model.tenants tenantList

            _ ->
                panel model.selected (tenantDetail model)
        ]


tenantList : List Api.TenantRow -> Html Msg
tenantList rows =
    if List.isEmpty rows then
        Ui.card [ p [ class "muted" ] [ text "No accounts match." ] ]

    else
        Ui.card
            [ ul [ class "connected-list" ]
                (List.map
                    (\row ->
                        li [ class "connected-item" ]
                            [ span [ class "connected-name" ] [ text row.email ]
                            , span [ class "muted" ]
                                [ text (Maybe.withDefault "—" row.name) ]
                            , span [ class "pill" ] [ text row.plan ]
                            , Html.button
                                [ class "btn btn-ghost"
                                , Html.Attributes.type_ "button"
                                , onClick (OpenTenant row.id)
                                ]
                                [ text "Open" ]
                            ]
                    )
                    rows
                )
            ]


tenantDetail : Model -> Api.TenantDetail -> Html Msg
tenantDetail model detail =
    let
        confirmed =
            String.toLower (String.trim model.deleteConfirm)
                == String.toLower detail.tenant.email
    in
    div []
        [ Ui.card
            [ h2 [] [ text detail.tenant.email ]
            , p [ class "muted" ]
                [ text
                    (Maybe.withDefault "No business name" detail.tenant.name
                        ++ " · "
                        ++ detail.tenant.plan
                        ++ " · joined "
                        ++ String.left 10 detail.tenant.createdAt
                    )
                ]
            , p [ class "balance" ]
                [ text (Format.count "en-IN" detail.balance ++ " replies left") ]
            , p [ class "muted" ]
                [ text (Format.count "en-IN" detail.repliesUsed ++ " used · setup " ++ completed detail) ]
            , if List.isEmpty detail.whatsappNumbers then
                p [ class "muted" ] [ text "No WhatsApp number connected." ]

              else
                ul [ class "connected-list" ]
                    (List.map
                        (\number ->
                            li [ class "connected-item" ]
                                [ span [ class "connected-phone" ] [ text number ] ]
                        )
                        detail.whatsappNumbers
                    )
            , div [ class "card-actions" ]
                [ Ui.button
                    { label = "Back to the list"
                    , onClick = CloseTenant
                    , primary = False
                    , busy = False
                    }
                ]
            ]
        , Ui.card
            [ h2 [] [ text "Grant replies" ]
            , p [ class "muted" ]
                [ text "Goes onto the ledger as an operator grant, and shows up on the tenant's Credits tab." ]
            , Ui.field
                { id = "grant_count"
                , label = "How many"
                , value = model.grantCount
                , hint = ""
                , required = True
                , onInput = SetGrantCount
                }
            , Ui.field
                { id = "grant_expiry"
                , label = "Expires in (days)"
                , value = model.grantExpiryDays
                , hint = "Leave empty and they never expire."
                , required = False
                , onInput = SetGrantExpiry
                }
            , div [ class "card-actions" ]
                [ Ui.button
                    { label = "Grant"
                    , onClick = GrantReplies
                    , primary = True
                    , busy = model.saving
                    }
                ]
            ]
        , Ui.card
            [ h2 [] [ text "Delete this account" ]
            , p []
                [ text "Wipes their settings, number, persona and message metadata. Payment rows stay, with the account identifier removed. This cannot be undone." ]
            , Ui.field
                { id = "tenant_delete_confirm"
                , label = "Type " ++ detail.tenant.email ++ " to confirm"
                , value = model.deleteConfirm
                , hint = ""
                , required = False
                , onInput = SetDeleteConfirm
                }
            , div [ class "card-actions" ]
                [ Html.button
                    [ class "btn btn-danger"
                    , Html.Attributes.type_ "button"
                    , Html.Attributes.disabled (not confirmed || model.saving)
                    , onClick DeleteTenant
                    ]
                    [ text "Delete permanently" ]
                ]
            ]
        , auditList detail.audit
        ]


completed : Api.TenantDetail -> String
completed detail =
    if detail.onboardingComplete then
        "finished"

    else
        "unfinished"


auditTab : Model -> Html Msg
auditTab model =
    section []
        [ Ui.card
            [ h2 [] [ text "Audit log" ]
            , Ui.field
                { id = "audit_actor"
                , label = "Actor"
                , value = model.auditActor
                , hint = "Partial email; case-insensitive."
                , required = False
                , onInput = SetAuditActor
                }
            , Ui.field
                { id = "audit_action"
                , label = "Action"
                , value = model.auditAction
                , hint = "Exact, e.g. update_pricing."
                , required = False
                , onInput = SetAuditAction
                }
            , div [ class "card-actions" ]
                [ Ui.button
                    { label = "Search"
                    , onClick = SearchAudit
                    , primary = True
                    , busy = False
                    }
                ]
            ]
        , panel model.audit auditList
        ]


auditList : List Api.AuditEntry -> Html Msg
auditList entries =
    Ui.card
        [ h3 [] [ text "Recent actions" ]
        , if List.isEmpty entries then
            p [ class "muted" ] [ text "Nothing recorded." ]

          else
            table [ class "rate-table" ]
                [ thead []
                    [ tr []
                        [ th [] [ text "When" ]
                        , th [] [ text "Who" ]
                        , th [] [ text "What" ]
                        ]
                    ]
                , tbody []
                    (List.map
                        (\entry ->
                            tr []
                                [ td [ class "mono" ] [ text (String.left 16 entry.at) ]
                                , td [] [ text entry.actor ]
                                , td []
                                    [ text entry.action
                                    , p [ class "hint" ]
                                        [ text
                                            (entry.resourceType
                                                ++ (case entry.resourceId of
                                                        Just id ->
                                                            " " ++ id

                                                        Nothing ->
                                                            ""
                                                   )
                                            )
                                        ]
                                    ]
                                ]
                        )
                        entries
                    )
                ]
        ]
