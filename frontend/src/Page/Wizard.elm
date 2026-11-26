module Page.Wizard exposing (Model, Msg, init, subscriptions, update, view)

{-| The onboarding wizard: business basics, connect WhatsApp, launch.

The server owns which step you're on. Every advance is a write that validates
the step being left, and the response carries the new step — so this module
never decides progression, it renders whatever `/api/wizard` last said.

-}

import Api
import Format
import Html exposing (Html, div, h1, h2, li, ol, p, section, span, text, ul)
import Html.Attributes exposing (attribute, class, classList)
import Html.Events
import Json.Encode as E
import Ports
import RemoteData exposing (RemoteData(..))
import Ui



-- MODEL


type alias Model =
    { wizard : Api.Data Api.WizardView
    , basics : Api.Basics
    , saving : Bool
    , connectingMeta : Bool
    , problem : Maybe String
    }


init : Maybe String -> ( Model, Cmd Msg )
init initialProblem =
    ( { wizard = Loading
      , basics = Api.emptyBasics
      , saving = False
      , connectingMeta = False
      , problem = initialProblem
      }
    , Api.getWizard GotWizard
    )



-- UPDATE


type Msg
    = GotWizard (Api.Data Api.WizardView)
    | RefreshedWizard (Api.Data Api.WizardView)
    | EditBasics (Api.Basics -> Api.Basics)
    | SaveBasics
    | StartMetaSignup Api.MetaSignup
    | MetaSignupCame E.Value
    | ChannelsDone
    | GoBack String
    | Finish


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotWizard result ->
            ( { model
                | wizard = result
                , saving = False
                , connectingMeta = False
                , problem =
                    case result of
                        Failure err ->
                            if Api.isUnauthenticated err then
                                Nothing

                            else
                                Just (Api.errorMessage err)

                        _ ->
                            model.problem
              }
                |> seedForm result
            , Cmd.none
            )

        RefreshedWizard result ->
            case result of
                Success wizard ->
                    ( { model | wizard = Success wizard } |> seedForm result, Cmd.none )

                Failure err ->
                    if Api.isUnauthenticated err then
                        ( { model | wizard = Failure err }, Cmd.none )

                    else
                        ( model, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        EditBasics change ->
            ( { model | basics = change model.basics }, Cmd.none )

        SaveBasics ->
            ( { model | saving = True, problem = Nothing }
            , Api.saveBasics model.basics GotWizard
            )

        StartMetaSignup signup ->
            ( { model | connectingMeta = True, problem = Nothing }
            , Ports.openMetaSignup
                { appId = signup.appId
                , configId = signup.configId
                , state = signup.state
                }
            )

        MetaSignupCame raw ->
            case Ports.decodeMetaSignupOutcome raw of
                Ports.MetaSignupOk ->
                    ( { model | connectingMeta = False, saving = True, problem = Nothing }
                    , Api.getWizard GotWizard
                    )

                Ports.MetaSignupCancelled ->
                    ( { model
                        | connectingMeta = False
                        , problem = Just "WhatsApp signup was cancelled. Click Connect with Meta whenever you're ready."
                      }
                    , Api.getWizard RefreshedWizard
                    )

                Ports.MetaSignupError errMsg ->
                    ( { model
                        | connectingMeta = False
                        , problem = Just errMsg
                      }
                    , Api.getWizard RefreshedWizard
                    )

        ChannelsDone ->
            ( { model | saving = True, problem = Nothing }, Api.channelsDone GotWizard )

        GoBack step ->
            ( { model | saving = True, problem = Nothing }, Api.stepWizard step GotWizard )

        Finish ->
            ( { model | saving = True, problem = Nothing }, Api.completeWizard GotWizard )


seedForm : Api.Data Api.WizardView -> Model -> Model
seedForm result model =
    case result of
        Success wizard ->
            { model | basics = wizard.basics }

        _ ->
            model


subscriptions : Sub Msg
subscriptions =
    Ports.metaSignupOutcome MetaSignupCame



-- VIEW


wizardSteps : List String
wizardSteps =
    [ "basics", "channels", "launch" ]


view : Model -> Html Msg
view model =
    div [ class "wizard" ]
        [ Ui.remote model.wizard (body model) ]


body : Model -> Api.WizardView -> Html Msg
body model wizard =
    div []
        [ progress wizard
        , case model.problem of
            Just message ->
                Ui.banner "error" message

            Nothing ->
                text ""
        , case wizard.step of
            "basics" ->
                basicsStep model

            "channels" ->
                channelsStep model wizard

            "launch" ->
                launchStep model wizard

            other ->
                Ui.banner "error" ("Unknown setup step: " ++ other)
        ]


progress : Api.WizardView -> Html Msg
progress wizard =
    let
        currentIndex =
            indexOf wizard.step wizardSteps
    in
    ol [ class "wizard-progress" ]
        (List.indexedMap
            (\i step ->
                li
                    [ classList
                        [ ( "is-done", i < currentIndex )
                        , ( "is-current", i == currentIndex )
                        ]
                    , attribute "aria-current"
                        (if i == currentIndex then
                            "step"

                         else
                            "false"
                        )
                    ]
                    [ span [ class "wizard-step-index" ] [ text (String.fromInt (i + 1)) ]
                    , span [ class "wizard-step-label" ] [ text (stepLabel step) ]
                    ]
            )
            wizardSteps
        )


indexOf : String -> List String -> Int
indexOf needle haystack =
    haystack
        |> List.indexedMap Tuple.pair
        |> List.filter (\( _, s ) -> s == needle)
        |> List.head
        |> Maybe.map Tuple.first
        |> Maybe.withDefault 0


stepLabel : String -> String
stepLabel step =
    case step of
        "basics" ->
            "Your business"

        "channels" ->
            "Connect WhatsApp"

        "launch" ->
            "Go live"

        other ->
            other



-- STEP: BASICS


basicsStep : Model -> Html Msg
basicsStep model =
    let
        b =
            model.basics
    in
    section [ class "wizard-step" ]
        [ h1 [] [ text "Tell us about your business" ]
        , p [ class "step-sub" ]
            [ text "These details go on your invoices and help replybots greet your WhatsApp customers properly." ]
        , Ui.card
            [ Ui.field
                { id = "name"
                , label = "Brand name"
                , value = b.brandName
                , hint = "What customers call you."
                , required = True
                , onInput = \v -> EditBasics (\x -> { x | brandName = v })
                }
            , Ui.field
                { id = "contact_name"
                , label = "Your name"
                , value = b.contactName
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | contactName = v })
                }
            , entityPicker b.entityType
            , Ui.field
                { id = "pan"
                , label = "PAN"
                , value = b.pan
                , hint = "Optional, but needed on a GST invoice."
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | pan = v })
                }
            , Ui.field
                { id = "gstin"
                , label = "GSTIN"
                , value = b.gstin
                , hint = "Optional."
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | gstin = v })
                }
            , Ui.field
                { id = "address"
                , label = "Address"
                , value = b.address
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | address = v })
                }
            , Ui.field
                { id = "state"
                , label = "State"
                , value = b.state
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | state = v })
                }
            , Ui.field
                { id = "pincode"
                , label = "PIN code"
                , value = b.pin
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | pin = v })
                }
            ]
        , div [ class "wizard-actions" ]
            [ Ui.button
                { label = "Continue"
                , onClick = SaveBasics
                , primary = True
                , busy = model.saving
                }
            ]
        ]


entityPicker : String -> Html Msg
entityPicker current =
    let
        options =
            [ ( "individual", "Individual" )
            , ( "sole_proprietorship", "Sole proprietorship" )
            , ( "partnership", "Partnership" )
            , ( "pvt_ltd", "Private limited" )
            , ( "llp", "LLP" )
            ]
    in
    div [ class "field" ]
        [ Html.label [] [ text "Registered as", span [ class "required" ] [ text " *" ] ]
        , div [ class "choice-row" ]
            (List.map
                (\( wire, label_ ) ->
                    Html.button
                        [ classList [ ( "choice", True ), ( "is-selected", current == wire ) ]
                        , Html.Attributes.type_ "button"
                        , Html.Events.onClick (EditBasics (\x -> { x | entityType = wire }))
                        ]
                        [ text label_ ]
                )
                options
            )
        ]



-- STEP: CHANNELS (WHATSAPP ONLY)


channelsStep : Model -> Api.WizardView -> Html Msg
channelsStep model wizard =
    section [ class "wizard-step" ]
        [ h1 [] [ text "Connect your WhatsApp Business number" ]
        , p [ class "step-sub" ]
            [ text "Link your WhatsApp Business number through Meta Embedded Signup. Nothing changes for customers who already message you." ]
        , whatsappBlock model wizard
        , div [ class "wizard-actions" ]
            ([ Ui.button
                { label = "Back"
                , onClick = GoBack "basics"
                , primary = False
                , busy = model.saving || model.connectingMeta
                }
             ]
                ++ (if List.isEmpty wizard.whatsapp then
                        []

                    else
                        [ Ui.button
                            { label = "Continue"
                            , onClick = ChannelsDone
                            , primary = True
                            , busy = model.saving || model.connectingMeta
                            }
                        ]
                   )
            )
        ]


whatsappBlock : Model -> Api.WizardView -> Html Msg
whatsappBlock model wizard =
    Ui.card
        [ h2 [] [ text "WhatsApp Business" ]
        , if List.isEmpty wizard.whatsapp then
            text ""

          else
            ul [ class "connected-list" ]
                (List.map
                    (\account ->
                        li [ class "connected-item" ]
                            [ span [ class "connected-name" ] [ text account.name ]
                            , span [ class "connected-phone" ] [ text account.displayPhoneNumber ]
                            , span [ class "pill pill-ok" ] [ text "Connected" ]
                            ]
                    )
                    wizard.whatsapp
                )
        , case wizard.signup of
            Just signup ->
                div [ class "card-actions" ]
                    [ Ui.button
                        { label =
                            if List.isEmpty wizard.whatsapp then
                                "Connect with Meta"

                            else
                                "Connect another number"
                        , onClick = StartMetaSignup signup
                        , primary = List.isEmpty wizard.whatsapp
                        , busy = model.connectingMeta || model.saving
                        }
                    ]

            Nothing ->
                if List.isEmpty wizard.whatsapp then
                    Ui.banner "info"
                        "WhatsApp signup isn't configured on this deployment yet. Ask your operator to set META_APP_ID and WHATSAPP_SIGNUP_CONFIG_ID."

                else
                    text ""
        ]



-- STEP: LAUNCH


launchStep : Model -> Api.WizardView -> Html Msg
launchStep model wizard =
    let
        unitPrice =
            Api.unitPriceFor wizard.pricing.defaultCurrency wizard.pricing
    in
    section [ class "wizard-step" ]
        [ h1 [] [ text "Ready to launch" ]
        , p [ class "step-sub" ]
            [ text
                (if String.isEmpty wizard.basics.brandName then
                    "Review your setup and go live on WhatsApp."

                 else
                    "Review your setup for "
                        ++ wizard.basics.brandName
                        ++ " and go live on WhatsApp."
                )
            ]
        , Ui.card
            [ h2 [] [ text "Setup summary" ]
            , p []
                [ text
                    ("Business: "
                        ++ (if String.isEmpty wizard.basics.brandName then
                                "—"

                            else
                                wizard.basics.brandName
                           )
                    )
                ]
            , ul [ class "connected-list" ]
                (List.map
                    (\account ->
                        li [ class "connected-item" ]
                            [ span [ class "connected-name" ] [ text account.name ]
                            , span [ class "connected-phone" ] [ text account.displayPhoneNumber ]
                            , span [ class "pill pill-ok" ] [ text "Connected" ]
                            ]
                    )
                    wizard.whatsapp
                )
            ]
        , Ui.card
            [ h2 [] [ text "Prepaid credits" ]
            , p []
                [ text
                    ("1 credit = 1 reply"
                        ++ (if unitPrice > 0 then
                                " (from " ++ Format.milliRate "en-IN" wizard.pricing.defaultCurrency unitPrice ++ " per reply)"

                            else
                                ""
                           )
                        ++ ". Current balance: "
                        ++ Format.count "en-IN" wizard.creditsBalance
                        ++ " credits."
                    )
                ]
            , p [ class "muted" ]
                [ text "You can top up anytime in the Credits tab right after launching." ]
            ]
        , div [ class "wizard-actions" ]
            [ Ui.button
                { label = "Back"
                , onClick = GoBack "channels"
                , primary = False
                , busy = model.saving
                }
            , Ui.button
                { label = "Go live"
                , onClick = Finish
                , primary = True
                , busy = model.saving
                }
            ]
        ]
