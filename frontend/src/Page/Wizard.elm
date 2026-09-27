module Page.Wizard exposing (Model, Msg, init, update, view)

{-| The onboarding wizard: basics, connect WhatsApp, pick a persona, launch.

The server owns which step you're on. Every advance is a write that validates
the step being left, and the response carries the new step — so this module
never decides progression, it renders whatever `/api/wizard` last said. That's
why there's no local `currentStep` field: two sources of truth for "where am
I" is exactly how a wizard ends up letting someone skip a step.

Form state is local and seeded from the server's copy on load. Edits stay
local until you press the button that advances the step.

-}

import Api
import Format
import Html exposing (Html, div, h1, h2, li, ol, p, section, span, text, ul)
import Html.Attributes exposing (attribute, class, classList)
import Html.Events
import RemoteData exposing (RemoteData(..))
import Ui



-- MODEL


type alias Model =
    { wizard : Api.Data Api.Wizard

    -- Local edits for the step being filled in. Seeded from the server's
    -- copy whenever a fresh wizard arrives.
    , basics : Api.Business
    , personaSlug : String
    , goal : String
    , goalUrl : String
    , handoffText : String

    -- Set while a mutation is in flight so buttons can't be double-fired.
    , saving : Bool

    -- A validation message from the last rejected save.
    , problem : Maybe String
    }


init : ( Model, Cmd Msg )
init =
    ( { wizard = Loading
      , basics = emptyBusiness
      , personaSlug = ""
      , goal = ""
      , goalUrl = ""
      , handoffText = ""
      , saving = False
      , problem = Nothing
      }
    , Api.getWizard GotWizard
    )


emptyBusiness : Api.Business
emptyBusiness =
    { name = ""
    , contactName = ""
    , phone = ""
    , businessType = ""
    , pan = ""
    , gstin = ""
    , address = ""
    , state = ""
    , pincode = ""
    }



-- UPDATE


type Msg
    = GotWizard (Api.Data Api.Wizard)
    | EditBasics (Api.Business -> Api.Business)
    | SaveBasics
    | PickPersona String
    | EditGoal String
    | EditGoalUrl String
    | EditHandoff String
    | SavePersona
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
                , problem =
                    case result of
                        Failure err ->
                            if Api.isUnauthenticated err then
                                -- Main routes to login on this; showing a
                                -- message too would flash and vanish.
                                Nothing

                            else
                                Just (Api.errorMessage err)

                        _ ->
                            Nothing
              }
                |> seedForm result
            , Cmd.none
            )

        EditBasics change ->
            ( { model | basics = change model.basics }, Cmd.none )

        SaveBasics ->
            ( { model | saving = True, problem = Nothing }
            , Api.saveBasics model.basics GotWizard
            )

        PickPersona slug ->
            ( { model | personaSlug = slug }, Cmd.none )

        EditGoal value ->
            ( { model | goal = value }, Cmd.none )

        EditGoalUrl value ->
            ( { model | goalUrl = value }, Cmd.none )

        EditHandoff value ->
            ( { model | handoffText = value }, Cmd.none )

        SavePersona ->
            ( { model | saving = True, problem = Nothing }
            , Api.savePersonaStep
                { archetypeSlug = model.personaSlug
                , goal = model.goal
                , goalUrl = model.goalUrl
                , handoffConditions = lines model.handoffText
                }
                GotWizard
            )

        ChannelsDone ->
            ( { model | saving = True, problem = Nothing }, Api.channelsDone GotWizard )

        GoBack step ->
            ( { model | saving = True, problem = Nothing }, Api.stepWizard step GotWizard )

        Finish ->
            ( { model | saving = True, problem = Nothing }, Api.completeWizard GotWizard )


{-| Copy the server's values into the local form fields, but only for fields
the user hasn't started editing — a re-fetch mid-typing shouldn't wipe input.
Since edits and fetches don't interleave in this flow (every save is a full
round-trip), seeding on every successful load is safe and keeps it simple.
-}
seedForm : Api.Data Api.Wizard -> Model -> Model
seedForm result model =
    case result of
        Success wizard ->
            { model
                | basics = wizard.business
                , personaSlug = wizard.persona.archetypeSlug
                , goal = wizard.persona.goal
                , goalUrl = wizard.persona.goalUrl
                , handoffText = String.join "\n" wizard.persona.handoffConditions
            }

        _ ->
            model


{-| Split a textarea into non-empty trimmed lines.
-}
lines : String -> List String
lines text_ =
    text_
        |> String.lines
        |> List.map String.trim
        |> List.filter (not << String.isEmpty)



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "wizard" ]
        [ Ui.remote model.wizard (body model) ]


body : Model -> Api.Wizard -> Html Msg
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

            "persona" ->
                personaStep model wizard

            "launch" ->
                launchStep model wizard

            other ->
                Ui.banner "error" ("Unknown setup step: " ++ other)
        ]


progress : Api.Wizard -> Html Msg
progress wizard =
    let
        currentIndex =
            indexOf wizard.step wizard.steps
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
            wizard.steps
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

        "persona" ->
            "Choose a voice"

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
            [ text "This is what goes on your invoices, and the name your customers will see in replies." ]
        , Ui.card
            [ Ui.field
                { id = "name"
                , label = "Brand name"
                , value = b.name
                , hint = "What customers call you."
                , required = True
                , onInput = \v -> EditBasics (\x -> { x | name = v })
                }
            , Ui.field
                { id = "contact_name"
                , label = "Your name"
                , value = b.contactName
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | contactName = v })
                }
            , Ui.field
                { id = "phone"
                , label = "Contact phone"
                , value = b.phone
                , hint = "For us to reach you — not the WhatsApp number you'll connect next."
                , required = True
                , onInput = \v -> EditBasics (\x -> { x | phone = v })
                }
            , entityPicker b.businessType
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
                , value = b.pincode
                , hint = ""
                , required = False
                , onInput = \v -> EditBasics (\x -> { x | pincode = v })
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
            [ ( "sole_proprietorship", "Sole proprietorship" )
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
                        , Html.Events.onClick (EditBasics (\x -> { x | businessType = wire }))
                        ]
                        [ text label_ ]
                )
                options
            )
        ]



-- STEP: CHANNELS


channelsStep : Model -> Api.Wizard -> Html Msg
channelsStep model wizard =
    section [ class "wizard-step" ]
        [ h1 [] [ text "Connect your WhatsApp number" ]
        , p [ class "step-sub" ]
            [ text "You'll go through Meta's own signup. Keep your existing number — nothing changes for the people who already message you." ]
        , if List.isEmpty wizard.whatsapp then
            Ui.card
                [ case wizard.signup of
                    Just signup ->
                        div []
                            [ p [] [ text "Meta will ask you to pick the business and number to connect." ]

                            -- The Embedded Signup SDK owns this button: it
                            -- opens Meta's dialog and redirects back to
                            -- /whatsapp/signup/*, which is a server route.
                            -- Rendering it as a plain link keeps the flow
                            -- working even when the SDK fails to load.
                            , Ui.linkButton
                                { label = "Connect with Meta"
                                , url = signupUrl signup
                                , primary = True
                                }
                            ]

                    Nothing ->
                        Ui.banner "info"
                            "WhatsApp signup isn't configured on this deployment yet. Ask your operator to set META_APP_ID and WHATSAPP_SIGNUP_CONFIG_ID."
                ]

          else
            div []
                [ ul [ class "connected-list" ]
                    (List.map
                        (\account ->
                            li [ class "connected-item" ]
                                [ span [ class "connected-name" ] [ text account.name ]
                                , span [ class "connected-phone" ] [ text account.phoneNumber ]
                                , span [ class "pill pill-ok" ] [ text "Connected" ]
                                ]
                        )
                        wizard.whatsapp
                    )
                , div [ class "wizard-actions" ]
                    [ Ui.button
                        { label = "Back"
                        , onClick = GoBack "basics"
                        , primary = False
                        , busy = model.saving
                        }
                    , Ui.button
                        { label = "Continue"
                        , onClick = ChannelsDone
                        , primary = True
                        , busy = model.saving
                        }
                    ]
                ]
        ]


{-| Meta's Embedded Signup dialog URL.

Built here rather than by the SDK so the button is a real link: if
`connect.facebook.net` is blocked the flow still starts. `state` is the
one-shot nonce the callback checks.

-}
signupUrl : Api.Signup -> String
signupUrl signup =
    "https://www.facebook.com/v21.0/dialog/oauth"
        ++ ("?client_id=" ++ signup.appId)
        ++ ("&config_id=" ++ signup.configId)
        ++ ("&state=" ++ signup.state)
        ++ "&response_type=code&override_default_response_type=true"
        ++ "&redirect_uri="
        ++ percentEncodedCallback


{-| The signup callback, percent-encoded for the `redirect_uri` parameter.
Relative URLs aren't allowed there, so it's resolved against the current
origin at runtime by the worker's own redirect handling.
-}
percentEncodedCallback : String
percentEncodedCallback =
    "%2Fwhatsapp%2Fsignup%2Fcallback"



-- STEP: PERSONA


personaStep : Model -> Api.Wizard -> Html Msg
personaStep model wizard =
    section [ class "wizard-step" ]
        [ h1 [] [ text "Choose how it should sound" ]
        , p [ class "step-sub" ]
            [ text "Pick a starting voice. You can rewrite every word of it later from your dashboard." ]
        , Ui.card
            [ voicePicker model.personaSlug
            , Ui.field
                { id = "goal"
                , label = "What should each conversation aim for?"
                , value = model.goal
                , hint = "For example: get them to book a slot, or send them to the menu."
                , required = False
                , onInput = EditGoal
                }
            , Ui.field
                { id = "goal_url"
                , label = "A link worth sending"
                , value = model.goalUrl
                , hint = "Optional. Your booking page, menu, or catalogue."
                , required = False
                , onInput = EditGoalUrl
                }
            , Ui.textarea
                { id = "handoff"
                , label = "When should it stop and fetch you?"
                , value = model.handoffText
                , hint = "One per line. Prices and commitments already stop it automatically — these are your own additions."
                , rows = 4
                , onInput = EditHandoff
                }
            ]
        , safetyNote wizard.persona.safetyStatus
        , div [ class "wizard-actions" ]
            [ Ui.button
                { label = "Back"
                , onClick = GoBack "channels"
                , primary = False
                , busy = model.saving
                }
            , Ui.button
                { label = "Continue"
                , onClick = SavePersona
                , primary = True
                , busy = model.saving
                }
            ]
        ]


{-| The four shipped archetypes. Labels and blurbs are duplicated from the
seeded catalog rather than fetched: the picker has to render before
`/api/archetypes` could answer, and an operator adding a fifth archetype
surfaces it on the dashboard's persona editor, which does fetch the catalog.
-}
voicePicker : String -> Html Msg
voicePicker current =
    let
        voices =
            [ ( "friendly", "Friendly", "Warm and familiar, like a shopkeeper who knows you." )
            , ( "professional", "Professional", "Brief and businesslike. Confirms what's possible." )
            , ( "playful", "Playful", "Upbeat, a little emoji, never cutesy." )
            , ( "formal", "Formal", "Polite and measured. Addresses customers respectfully." )
            ]
    in
    div [ class "field" ]
        [ Html.label [] [ text "Voice" ]
        , ul [ class "voice-grid" ]
            (List.map
                (\( slug, label_, blurb ) ->
                    li []
                        [ Html.button
                            [ classList [ ( "voice-card", True ), ( "is-selected", current == slug ) ]
                            , Html.Attributes.type_ "button"
                            , Html.Events.onClick (PickPersona slug)
                            ]
                            [ h2 [] [ text label_ ]
                            , p [] [ text blurb ]
                            ]
                        ]
                )
                voices
            )
        ]


safetyNote : String -> Html Msg
safetyNote status =
    case status of
        "pending" ->
            Ui.banner "info"
                "We check every prompt before it's allowed to answer customers. It usually takes a few seconds."

        "rejected" ->
            Ui.banner "error"
                "That prompt didn't pass our safety check. Try a different voice, or edit it from your dashboard."

        _ ->
            text ""



-- STEP: LAUNCH


launchStep : Model -> Api.Wizard -> Html Msg
launchStep model wizard =
    let
        l =
            wizard.launch
    in
    section [ class "wizard-step" ]
        [ h1 [] [ text "One last thing" ]
        , Ui.card
            [ h2 [] [ text "What you'll pay" ]
            , p []
                [ text "Every AI reply costs "
                , span [ class "price" ]
                    [ text (Format.milliRate "en-IN" l.currency l.unitPriceMilli) ]
                , text ". Canned replies are free, and you're never charged for a message Concierge decides not to answer."
                ]
            ]
        , if l.metered && not l.verified then
            Ui.card
                [ h2 [] [ text "Verify your card" ]
                , p []
                    [ text "We charge "
                    , span [ class "price" ]
                        [ text (Format.money "en-IN" l.currency l.verificationAmount) ]
                    , text " and refund it straight away. It's how we keep throwaway signups out."
                    ]
                , Ui.linkButton
                    { label = "Verify now"
                    , url = "/dashboard/billing"
                    , primary = True
                    }
                ]

          else
            text ""
        , div [ class "wizard-actions" ]
            [ Ui.button
                { label = "Back"
                , onClick = GoBack "persona"
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
