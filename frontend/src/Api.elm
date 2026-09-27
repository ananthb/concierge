module Api exposing
    ( ApiError(..)
    , Archetype
    , Billing
    , Bootstrap
    , Business
    , Credit
    , Data
    , DemoConfig
    , DemoPersona
    , DemoReply
    , Order
    , Persona
    , PersonaBuilder
    , Pricing
    , Reply
    , Session
    , Signup
    , WhatsAppAccount
    , Wizard
    , WizardPersona
    , channelsDone
    , completeWizard
    , delete
    , deleteAccount
    , demoChat
    , emptyBuilder
    , errorMessage
    , get
    , getArchetypes
    , getBilling
    , getBootstrap
    , getPersona
    , getWhatsApp
    , getWizard
    , isUnauthenticated
    , post
    , previewPersona
    , put
    , saveBasics
    , savePersona
    , savePersonaStep
    , saveWhatsApp
    , startCheckout
    , startVerification
    , stepWizard
    , verifyPayment
    )

{-| The single place that knows the shape of `/api/*`.

Two conventions the whole module rests on:

**Cookies are the credential.** The SPA and the worker are always on the same
origin, so the `HttpOnly` session cookie set by `/auth/callback` travels on
every request automatically. There is no token to store, attach or refresh,
and nothing here reads or writes `document.cookie`.

**Every mutation carries `X-Concierge-Request`.** The cookie is
`SameSite=Lax`, which already stops a cross-site form post, and a custom
header can't be set cross-origin without a preflight the worker never grants.
That pair replaces the CSRF token the old HTML forms posted. [`post`], [`put`]
and [`delete`] set it; [`get`] doesn't need it.

Errors decode the worker's `{"error": {"code", "message"}}` envelope, because
`Http.Error` on its own would reduce a useful "Connect a WhatsApp number to
continue." to `BadStatus 400`. See [`ApiError`].

-}

import Http
import Json.Decode as D exposing (Decoder)
import Json.Encode as E
import RemoteData exposing (RemoteData)



-- ERRORS


{-| What can go wrong with a call.

`Fault` is the worker answering in its own words — show `message` to the
user as-is. `Unauthenticated` is a 401, which the app treats as "the session
is gone, render the login page" rather than as an error to display.
`Transport` is everything else: offline, DNS, a 500 with no envelope.

-}
type ApiError
    = Fault { code : String, message : String }
    | Unauthenticated
    | Transport Http.Error


{-| Remote data whose failure carries our error type rather than `Http.Error`.
-}
type alias Data a =
    RemoteData ApiError a


{-| Human-readable text for an error, for rendering in a banner.
-}
errorMessage : ApiError -> String
errorMessage err =
    case err of
        Fault { message } ->
            message

        Unauthenticated ->
            "Your session has expired. Sign in again to continue."

        Transport (Http.BadStatus code) ->
            "The server returned an unexpected error (" ++ String.fromInt code ++ ")."

        Transport Http.NetworkError ->
            "Couldn't reach the server. Check your connection and try again."

        Transport Http.Timeout ->
            "That took too long. Try again."

        Transport (Http.BadBody detail) ->
            -- A decoder mismatch is our bug, not the user's. Say so plainly
            -- rather than showing them a decoder trace.
            "The server sent something we couldn't read. " ++ detail

        Transport (Http.BadUrl url) ->
            "Tried to call an invalid URL: " ++ url


{-| True when the failure means "not signed in", which callers handle by
routing to the login page instead of showing a message.
-}
isUnauthenticated : ApiError -> Bool
isUnauthenticated err =
    err == Unauthenticated



-- REQUESTS


{-| Header marking a request as coming from our own frontend.
-}
requestHeader : Http.Header
requestHeader =
    Http.header "X-Concierge-Request" "1"


expectJson : (Data a -> msg) -> Decoder a -> Http.Expect msg
expectJson toMsg decoder =
    Http.expectStringResponse (RemoteData.fromResult >> toMsg) <|
        \response ->
            case response of
                Http.BadUrl_ url ->
                    Err (Transport (Http.BadUrl url))

                Http.Timeout_ ->
                    Err (Transport Http.Timeout)

                Http.NetworkError_ ->
                    Err (Transport Http.NetworkError)

                Http.BadStatus_ metadata body ->
                    if metadata.statusCode == 401 then
                        Err Unauthenticated

                    else
                        -- Prefer the worker's own message. A body without the
                        -- envelope (a platform 5xx, say) falls back to the
                        -- status code.
                        case D.decodeString faultDecoder body of
                            Ok fault ->
                                Err (Fault fault)

                            Err _ ->
                                Err (Transport (Http.BadStatus metadata.statusCode))

                Http.GoodStatus_ _ body ->
                    -- 204 has no body. Decoders for endpoints that answer 204
                    -- are built with `D.succeed`, which still needs valid JSON
                    -- to run against, so substitute an empty object.
                    let
                        payload =
                            if String.trim body == "" then
                                "{}"

                            else
                                body
                    in
                    case D.decodeString decoder payload of
                        Ok value ->
                            Ok value

                        Err err ->
                            Err (Transport (Http.BadBody (D.errorToString err)))


faultDecoder : Decoder { code : String, message : String }
faultDecoder =
    D.field "error"
        (D.map2 (\c m -> { code = c, message = m })
            (D.field "code" D.string)
            (D.field "message" D.string)
        )


get : String -> Decoder a -> (Data a -> msg) -> Cmd msg
get path decoder toMsg =
    Http.request
        { method = "GET"
        , headers = []
        , url = path
        , body = Http.emptyBody
        , expect = expectJson toMsg decoder
        , timeout = Nothing
        , tracker = Nothing
        }


post : String -> E.Value -> Decoder a -> (Data a -> msg) -> Cmd msg
post path body decoder toMsg =
    mutate "POST" path body decoder toMsg


put : String -> E.Value -> Decoder a -> (Data a -> msg) -> Cmd msg
put path body decoder toMsg =
    mutate "PUT" path body decoder toMsg


delete : String -> E.Value -> Decoder a -> (Data a -> msg) -> Cmd msg
delete path body decoder toMsg =
    mutate "DELETE" path body decoder toMsg


mutate : String -> String -> E.Value -> Decoder a -> (Data a -> msg) -> Cmd msg
mutate method path body decoder toMsg =
    Http.request
        { method = method
        , headers = [ requestHeader ]
        , url = path
        , body = Http.jsonBody body
        , expect = expectJson toMsg decoder
        , timeout = Nothing
        , tracker = Nothing
        }



-- BOOTSTRAP


{-| Everything the app needs for its first frame, in one request.
-}
type alias Bootstrap =
    { pricing : Pricing
    , demo : DemoConfig
    , session : Maybe Session
    }


type alias Pricing =
    { unitPriceMilli : Int
    , currency : String
    , minCredits : Int
    , maxCredits : Int
    , currencies : List { code : String, unitPriceMilli : Int }
    }


type alias DemoConfig =
    { enabled : Bool
    , maxUserTurns : Int
    , idleTimeoutSecs : Int
    , personas : List DemoPersona
    }


type alias DemoPersona =
    { slug : String
    , label : String
    , description : String
    , greeting : String

    -- The exact middle sent to the model, shown in the "view prompt" panel.
    , prompt : String
    , businessName : String
    }


type alias Session =
    { tenantId : String
    , email : String
    , name : Maybe String
    , currency : String
    , metered : Bool

    -- "wizard" until onboarding is sealed, then "dashboard".
    , destination : String
    , onboardingStep : Maybe String

    -- False while the persona's safety verdict is pending or rejected; AI
    -- replies are blocked until it clears.
    , aiReady : Bool
    }


getBootstrap : (Data Bootstrap -> msg) -> Cmd msg
getBootstrap =
    get "/api/bootstrap" bootstrapDecoder


bootstrapDecoder : Decoder Bootstrap
bootstrapDecoder =
    D.map3 Bootstrap
        (D.field "pricing" pricingDecoder)
        (D.field "demo" demoDecoder)
        (D.field "session" (D.nullable sessionDecoder))


pricingDecoder : Decoder Pricing
pricingDecoder =
    D.map5 Pricing
        (D.field "unit_price_milli" D.int)
        (D.field "currency" D.string)
        (D.field "min_credits" D.int)
        (D.field "max_credits" D.int)
        (D.field "currencies"
            (D.list
                (D.map2 (\c p -> { code = c, unitPriceMilli = p })
                    (D.field "code" D.string)
                    (D.field "unit_price_milli" D.int)
                )
            )
        )


demoDecoder : Decoder DemoConfig
demoDecoder =
    D.map4 DemoConfig
        (D.field "enabled" D.bool)
        (D.field "max_user_turns" D.int)
        (D.field "idle_timeout_secs" D.int)
        (D.field "personas" (D.list demoPersonaDecoder))


demoPersonaDecoder : Decoder DemoPersona
demoPersonaDecoder =
    D.map6 DemoPersona
        (D.field "slug" D.string)
        (D.field "label" D.string)
        (D.field "description" D.string)
        (D.field "greeting" D.string)
        (D.field "prompt" D.string)
        (D.at [ "business", "name" ] D.string |> withDefault "")


sessionDecoder : Decoder Session
sessionDecoder =
    D.map8 Session
        (D.field "tenant_id" D.string)
        (D.field "email" D.string)
        (D.field "name" (D.nullable D.string))
        (D.field "currency" D.string)
        (D.field "metered" D.bool)
        (D.field "destination" D.string)
        (D.field "onboarding_step" (D.nullable D.string))
        (D.field "ai_ready" D.bool)



-- DEMO CHAT


type alias DemoReply =
    { reply : String

    -- True once the conversation has been handed to a human; the client
    -- echoes it back on every later turn so the server keeps using the
    -- holding-pattern prompt.
    , handoff : Bool
    }


demoChat :
    { persona : String, messages : List { role : String, content : String }, handoff : Bool }
    -> (Data DemoReply -> msg)
    -> Cmd msg
demoChat args =
    post "/api/demo/chat"
        (E.object
            [ ( "persona", E.string args.persona )
            , ( "messages"
              , E.list
                    (\m ->
                        E.object
                            [ ( "role", E.string m.role )
                            , ( "content", E.string m.content )
                            ]
                    )
                    args.messages
              )
            , ( "handoff", E.bool args.handoff )
            ]
        )
        (D.map2 DemoReply
            (D.field "reply" D.string)
            (D.field "handoff" D.bool)
        )


{-| Close the account. Irreversible.

The worker requires the account's own email address echoed back in
`confirm_email`, so a mis-click can't do it. Answers 204 and clears the
session cookie; the app then reloads onto the landing page.

-}
deleteAccount : String -> (Data () -> msg) -> Cmd msg
deleteAccount confirmEmail =
    delete "/api/account"
        (E.object [ ( "confirm_email", E.string confirmEmail ) ])
        (D.succeed ())



-- WIZARD


type alias Wizard =
    { step : String
    , steps : List String
    , completed : Bool
    , business : Business
    , persona : WizardPersona
    , whatsapp : List WhatsAppAccount
    , signup : Maybe Signup
    , launch : Launch
    }


type alias Business =
    { name : String
    , contactName : String
    , phone : String
    , businessType : String
    , pan : String
    , gstin : String
    , address : String
    , state : String
    , pincode : String
    }


type alias WizardPersona =
    { archetypeSlug : String
    , goal : String
    , goalUrl : String
    , handoffConditions : List String
    , safetyStatus : String
    }


type alias Launch =
    { verified : Bool
    , verificationAmount : Int
    , unitPriceMilli : Int
    , currency : String
    , minCredits : Int
    , maxCredits : Int
    , metered : Bool
    }


{-| Meta Embedded Signup parameters. `state` is a one-shot nonce, so a fresh
one arrives with every wizard read.
-}
type alias Signup =
    { appId : String
    , configId : String
    , state : String
    }


getWizard : (Data Wizard -> msg) -> Cmd msg
getWizard =
    get "/api/wizard" wizardDecoder


{-| Save the basics step. Advances the server-side step on success.
-}
saveBasics : Business -> (Data Wizard -> msg) -> Cmd msg
saveBasics b =
    put "/api/wizard/basics"
        (E.object
            [ ( "name", E.string b.name )
            , ( "contact_name", E.string b.contactName )
            , ( "phone", E.string b.phone )
            , ( "business_type", E.string b.businessType )
            , ( "pan", E.string b.pan )
            , ( "gstin", E.string b.gstin )
            , ( "address", E.string b.address )
            , ( "state", E.string b.state )
            , ( "pincode", E.string b.pincode )
            ]
        )
        wizardDecoder


savePersonaStep :
    { archetypeSlug : String, goal : String, goalUrl : String, handoffConditions : List String }
    -> (Data Wizard -> msg)
    -> Cmd msg
savePersonaStep p =
    put "/api/wizard/persona"
        (E.object
            [ ( "archetype_slug", E.string p.archetypeSlug )
            , ( "goal", E.string p.goal )
            , ( "goal_url", E.string p.goalUrl )
            , ( "handoff_conditions", E.list E.string p.handoffConditions )
            ]
        )
        wizardDecoder


channelsDone : (Data Wizard -> msg) -> Cmd msg
channelsDone =
    post "/api/wizard/channels/done" (E.object []) wizardDecoder


{-| Step backwards. The server refuses a forward jump.
-}
stepWizard : String -> (Data Wizard -> msg) -> Cmd msg
stepWizard to =
    post "/api/wizard/step" (E.object [ ( "to", E.string to ) ]) wizardDecoder


completeWizard : (Data Wizard -> msg) -> Cmd msg
completeWizard =
    post "/api/wizard/complete" (E.object []) wizardDecoder


wizardDecoder : Decoder Wizard
wizardDecoder =
    D.map8 Wizard
        (D.field "step" D.string)
        (D.field "steps" (D.list D.string))
        (D.field "completed" D.bool)
        (D.field "business" businessDecoder)
        (D.field "persona" wizardPersonaDecoder)
        (D.field "whatsapp" (D.list whatsAppDecoder))
        (D.field "signup" (D.nullable signupDecoder))
        (D.field "launch" launchDecoder)


businessDecoder : Decoder Business
businessDecoder =
    D.map8 (\n c p b pan g a s -> Business n c p b pan g a s)
        (D.field "name" D.string)
        (D.field "contact_name" D.string)
        (D.field "phone" D.string)
        (D.field "business_type" D.string)
        (D.field "pan" D.string)
        (D.field "gstin" D.string)
        (D.field "address" D.string)
        (D.field "state" D.string)
        |> D.andThen (\f -> D.map f (D.field "pincode" D.string))


wizardPersonaDecoder : Decoder WizardPersona
wizardPersonaDecoder =
    D.map5 WizardPersona
        (D.field "archetype_slug" D.string)
        (D.field "goal" D.string)
        (D.field "goal_url" D.string)
        (D.field "handoff_conditions" (D.list D.string))
        (D.field "safety_status" D.string)


launchDecoder : Decoder Launch
launchDecoder =
    D.map7 Launch
        (D.field "verified" D.bool)
        (D.field "verification_amount" D.int)
        (D.field "unit_price_milli" D.int)
        (D.field "currency" D.string)
        (D.field "min_credits" D.int)
        (D.field "max_credits" D.int)
        (D.field "metered" D.bool)


signupDecoder : Decoder Signup
signupDecoder =
    D.map3 Signup
        (D.field "app_id" D.string)
        (D.field "config_id" D.string)
        (D.field "state" D.string)



-- WHATSAPP


type alias WhatsAppAccount =
    { id : String
    , name : String
    , phoneNumber : String
    , phoneNumberId : String
    , reply : Reply
    }


{-| `mode` is "canned" or "prompt"; `text` is the message or the instruction.
-}
type alias Reply =
    { enabled : Bool
    , mode : String
    , text : String
    , waitSeconds : Int
    }


getWhatsApp : (Data { accounts : List WhatsAppAccount, signup : Maybe Signup } -> msg) -> Cmd msg
getWhatsApp =
    get "/api/whatsapp"
        (D.map2 (\a s -> { accounts = a, signup = s })
            (D.field "accounts" (D.list whatsAppDecoder))
            (D.field "signup" (D.nullable signupDecoder))
        )


saveWhatsApp : String -> { name : String, reply : Reply } -> (Data WhatsAppAccount -> msg) -> Cmd msg
saveWhatsApp id args =
    put ("/api/whatsapp/" ++ id)
        (E.object
            [ ( "name", E.string args.name )
            , ( "reply"
              , E.object
                    [ ( "enabled", E.bool args.reply.enabled )
                    , ( "mode", E.string args.reply.mode )
                    , ( "text", E.string args.reply.text )
                    , ( "wait_seconds", E.int args.reply.waitSeconds )
                    ]
              )
            ]
        )
        whatsAppDecoder


whatsAppDecoder : Decoder WhatsAppAccount
whatsAppDecoder =
    D.map5 WhatsAppAccount
        (D.field "id" D.string)
        (D.field "name" D.string)
        (D.field "phone_number" D.string)
        (D.field "phone_number_id" D.string)
        (D.field "reply" replyDecoder)


replyDecoder : Decoder Reply
replyDecoder =
    D.map4 Reply
        (D.field "enabled" D.bool)
        (D.field "mode" D.string)
        (D.field "text" D.string)
        (D.field "wait_seconds" D.int)



-- PERSONA


type alias Persona =
    { mode : String
    , builder : PersonaBuilder
    , customPrompt : String
    , safetyStatus : String
    , safetyReason : Maybe String
    , aiReady : Bool
    , prompt : String
    , preamble : String
    , postamble : String
    , maxCustomPrompt : Int
    }


type alias PersonaBuilder =
    { archetypeSlug : String
    , bizName : String
    , bizType : String
    , city : String
    , hours : String
    , goal : String
    , goalUrl : String
    , catchPhrases : List String
    , offTopics : List String
    , never : String
    , handoffConditions : List String
    }


emptyBuilder : PersonaBuilder
emptyBuilder =
    { archetypeSlug = ""
    , bizName = ""
    , bizType = ""
    , city = ""
    , hours = ""
    , goal = ""
    , goalUrl = ""
    , catchPhrases = []
    , offTopics = []
    , never = ""
    , handoffConditions = []
    }


type alias Archetype =
    { slug : String
    , label : String
    , description : String
    , greeting : String
    }


getPersona : (Data Persona -> msg) -> Cmd msg
getPersona =
    get "/api/persona" personaDecoder


savePersona : { mode : String, builder : PersonaBuilder, customPrompt : String } -> (Data Persona -> msg) -> Cmd msg
savePersona args =
    put "/api/persona"
        (E.object
            [ ( "mode", E.string args.mode )
            , ( "builder", encodeBuilder args.builder )
            , ( "custom_prompt", E.string args.customPrompt )
            ]
        )
        personaDecoder


{-| Compose a prompt from unsaved fields. Writes nothing and never touches
the safety queue, so it's safe to call on every edit.
-}
previewPersona : PersonaBuilder -> (Data String -> msg) -> Cmd msg
previewPersona builder =
    post "/api/persona/preview"
        (E.object [ ( "builder", encodeBuilder builder ) ])
        (D.field "prompt" D.string)


getArchetypes : (Data (List Archetype) -> msg) -> Cmd msg
getArchetypes =
    get "/api/archetypes"
        (D.field "archetypes"
            (D.list
                (D.map4 Archetype
                    (D.field "slug" D.string)
                    (D.field "label" D.string)
                    (D.field "description" D.string)
                    (D.field "greeting" D.string)
                )
            )
        )


encodeBuilder : PersonaBuilder -> E.Value
encodeBuilder b =
    E.object
        [ ( "archetype_slug", E.string b.archetypeSlug )
        , ( "biz_name", E.string b.bizName )
        , ( "biz_type", E.string b.bizType )
        , ( "city", E.string b.city )
        , ( "hours", E.string b.hours )
        , ( "goal", E.string b.goal )
        , ( "goal_url", E.string b.goalUrl )
        , ( "catch_phrases", E.list E.string b.catchPhrases )
        , ( "off_topics", E.list E.string b.offTopics )
        , ( "never", E.string b.never )
        , ( "handoff_conditions", E.list E.string b.handoffConditions )
        ]


personaDecoder : Decoder Persona
personaDecoder =
    D.map8
        (\mode builder custom status reason ready prompt pre ->
            { mode = mode
            , builder = builder
            , customPrompt = custom
            , safetyStatus = status
            , safetyReason = reason
            , aiReady = ready
            , prompt = prompt
            , preamble = pre
            , postamble = ""
            , maxCustomPrompt = 2000
            }
        )
        (D.field "mode" D.string)
        (D.field "builder" builderDecoder)
        (D.field "custom_prompt" D.string)
        (D.at [ "safety", "status" ] D.string)
        (D.at [ "safety", "reason" ] (D.nullable D.string))
        (D.at [ "safety", "ai_ready" ] D.bool)
        (D.field "prompt" D.string)
        (D.field "preamble" D.string)
        |> D.andThen
            (\p ->
                D.map2 (\postText max -> { p | postamble = postText, maxCustomPrompt = max })
                    (D.field "postamble" D.string)
                    (D.field "max_custom_prompt" D.int)
            )


builderDecoder : Decoder PersonaBuilder
builderDecoder =
    D.map8
        (\slug name typ city hours goal url phrases ->
            { archetypeSlug = slug
            , bizName = name
            , bizType = typ
            , city = city
            , hours = hours
            , goal = goal
            , goalUrl = url
            , catchPhrases = phrases
            , offTopics = []
            , never = ""
            , handoffConditions = []
            }
        )
        (D.field "archetype_slug" D.string)
        (D.field "biz_name" D.string)
        (D.field "biz_type" D.string)
        (D.field "city" D.string)
        (D.field "hours" D.string)
        (D.field "goal" D.string)
        (D.field "goal_url" D.string)
        (D.field "catch_phrases" (D.list D.string))
        |> D.andThen
            (\b ->
                D.map3 (\off never handoff -> { b | offTopics = off, never = never, handoffConditions = handoff })
                    (D.field "off_topics" (D.list D.string))
                    (D.field "never" D.string)
                    (D.field "handoff_conditions" (D.list D.string))
            )



-- BILLING


type alias Billing =
    { balance : Int
    , repliesUsed : Int
    , credits : List Credit
    , unitPriceMilli : Int
    , currency : String
    , minCredits : Int
    , maxCredits : Int
    , metered : Bool
    , verified : Bool
    }


type alias Credit =
    { amount : Int
    , source : String
    , expiresAt : Maybe String
    }


{-| Razorpay checkout parameters. `keyId` is the publishable key; the secret
never leaves the worker.
-}
type alias Order =
    { orderId : String
    , amount : Int
    , currency : String
    , keyId : String
    , credits : Int
    }


getBilling : (Data Billing -> msg) -> Cmd msg
getBilling =
    get "/api/billing" billingDecoder


startCheckout : Int -> (Data Order -> msg) -> Cmd msg
startCheckout credits =
    post "/api/billing/checkout" (E.object [ ( "credits", E.int credits ) ]) orderDecoder


startVerification : (Data Order -> msg) -> Cmd msg
startVerification =
    post "/api/billing/verification" (E.object []) orderDecoder


{-| Confirm a completed checkout. Validates the signature only — credits are
granted by the Razorpay webhook, never by this call.
-}
verifyPayment :
    { orderId : String, paymentId : String, signature : String }
    -> (Data () -> msg)
    -> Cmd msg
verifyPayment args =
    post "/api/billing/verify"
        (E.object
            [ ( "razorpay_order_id", E.string args.orderId )
            , ( "razorpay_payment_id", E.string args.paymentId )
            , ( "razorpay_signature", E.string args.signature )
            ]
        )
        (D.succeed ())


orderDecoder : Decoder Order
orderDecoder =
    D.map5 Order
        (D.field "order_id" D.string)
        (D.field "amount" D.int)
        (D.field "currency" D.string)
        (D.field "key_id" D.string)
        (D.field "credits" D.int)


billingDecoder : Decoder Billing
billingDecoder =
    D.map8
        (\balance used credits price currency minC maxC metered ->
            { balance = balance
            , repliesUsed = used
            , credits = credits
            , unitPriceMilli = price
            , currency = currency
            , minCredits = minC
            , maxCredits = maxC
            , metered = metered
            , verified = False
            }
        )
        (D.field "balance" D.int)
        (D.field "replies_used" D.int)
        (D.field "credits" (D.list creditDecoder))
        (D.field "unit_price_milli" D.int)
        (D.field "currency" D.string)
        (D.field "min_credits" D.int)
        (D.field "max_credits" D.int)
        (D.field "metered" D.bool)
        |> D.andThen (\b -> D.map (\v -> { b | verified = v }) (D.field "verified" D.bool))


creditDecoder : Decoder Credit
creditDecoder =
    D.map3 Credit
        (D.field "amount" D.int)
        (D.field "source" D.string)
        (D.field "expires_at" (D.nullable D.string))



-- HELPERS


{-| A decoder that falls back instead of failing. Used for optional nested
fields where an absent value is normal rather than an error.
-}
withDefault : a -> Decoder a -> Decoder a
withDefault fallback decoder =
    D.oneOf [ decoder, D.succeed fallback ]
