module Api exposing
    ( ApiError(..)
    , Basics
    , BillingView
    , Bootstrap
    , ConversationBounds
    , ConversationSettings
    , CreditEntry
    , Data
    , DemoConfig
    , DemoPersona
    , DemoReply
    , MetaSignup
    , NotificationSettings
    , Order
    , Pricing
    , ReplyConfig
    , Session
    , SettingsView
    , WhatsAppAccount
    , WizardView
    , channelsDone
    , completeWizard
    , delete
    , deleteAccount
    , deleteWhatsApp
    , demoChat
    , emptyBasics
    , errorMessage
    , get
    , getBilling
    , getBootstrap
    , getSettings
    , getWhatsApp
    , getWizard
    , isUnauthenticated
    , post
    , put
    , saveBasics
    , saveConversation
    , saveLocale
    , saveNotifications
    , startCheckout
    , stepWizard
    , unitPriceFor
    , updateWhatsApp
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

import Dict exposing (Dict)
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
                        case D.decodeString faultDecoder body of
                            Ok fault ->
                                Err (Fault fault)

                            Err _ ->
                                Err (Transport (Http.BadStatus metadata.statusCode))

                Http.GoodStatus_ _ body ->
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
    { minCredits : Int
    , maxCredits : Int
    , unitPriceMilli : Dict String Int
    , defaultCurrency : String
    }


unitPriceFor : String -> Pricing -> Int
unitPriceFor currency pricing =
    case Dict.get currency pricing.unitPriceMilli of
        Just price ->
            price

        Nothing ->
            Dict.get pricing.defaultCurrency pricing.unitPriceMilli
                |> Maybe.withDefault 0


type alias DemoConfig =
    { enabled : Bool
    , maxUserTurns : Int
    , idleTimeoutSecs : Int
    , personas : List DemoPersona
    }


type alias DemoPersona =
    { id : String
    , businessName : String
    , businessType : String
    , city : String
    , hours : String
    , opener : String
    , summary : String
    }


type alias Session =
    { tenantId : String
    , email : String
    , name : Maybe String
    , locale : String
    , currency : String
    , destination : String
    , aiReady : Bool
    , safetyStatus : String
    , safetyReason : Maybe String
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
    D.oneOf
        [ D.map4 Pricing
            (D.field "min_credits" D.int)
            (D.field "max_credits" D.int)
            (D.field "unit_price_milli" (D.dict D.int))
            (D.field "default_currency" D.string)
        , D.map4
            (\minC maxC cur rates ->
                Pricing minC maxC (Dict.fromList (List.map (\r -> ( r.code, r.unitPriceMilli )) rates)) cur
            )
            (D.field "min_credits" D.int)
            (D.field "max_credits" D.int)
            (D.field "currency" D.string)
            (D.field "currencies"
                (D.list
                    (D.map2 (\c p -> { code = c, unitPriceMilli = p })
                        (D.field "code" D.string)
                        (D.field "unit_price_milli" D.int)
                    )
                )
            )
        , D.map4
            (\minC maxC unit cur ->
                Pricing minC maxC (Dict.singleton cur unit) cur
            )
            (D.field "min_credits" D.int)
            (D.field "max_credits" D.int)
            (D.field "unit_price_milli" D.int)
            (D.field "currency" D.string)
        ]


demoDecoder : Decoder DemoConfig
demoDecoder =
    D.map4 DemoConfig
        (D.field "enabled" D.bool)
        (D.field "max_user_turns" D.int)
        (D.field "idle_timeout_secs" D.int)
        (D.field "personas" (D.list demoPersonaDecoder))


demoPersonaDecoder : Decoder DemoPersona
demoPersonaDecoder =
    D.map7 DemoPersona
        (D.field "id" D.string)
        (D.field "business_name" D.string)
        (D.field "business_type" D.string)
        (D.field "city" D.string)
        (D.field "hours" D.string)
        (D.field "opener" D.string)
        (D.field "summary" D.string)


sessionDecoder : Decoder Session
sessionDecoder =
    D.map7
        (\tenantId email name locale currency destination aiReady ->
            { tenantId = tenantId
            , email = email
            , name = name
            , locale = locale
            , currency = currency
            , destination = destination
            , aiReady = aiReady
            , safetyStatus =
                if aiReady then
                    "approved"

                else
                    "pending"
            , safetyReason = Nothing
            }
        )
        (D.field "tenant_id" D.string)
        (D.field "email" D.string)
        (D.field "name" (D.nullable D.string))
        (D.field "locale" D.string)
        (D.field "currency" D.string)
        (D.field "destination" D.string)
        (D.field "ai_ready" D.bool)
        |> D.andThen
            (\s ->
                D.map2 (\status reason -> { s | safetyStatus = status, safetyReason = reason })
                    (withDefault s.safetyStatus (D.field "safety_status" D.string))
                    (withDefault Nothing (D.field "safety_reason" (D.nullable D.string)))
            )



-- DEMO CHAT


type alias DemoReply =
    { reply : String
    , handedOff : Bool
    }


demoChat :
    { personaId : String
    , history : List { role : String, content : String }
    , message : String
    , handoff : Bool
    }
    -> (Data DemoReply -> msg)
    -> Cmd msg
demoChat args =
    post "/api/demo/chat"
        (E.object
            [ ( "persona_id", E.string args.personaId )
            , ( "history"
              , E.list
                    (\m ->
                        E.object
                            [ ( "role", E.string m.role )
                            , ( "content", E.string m.content )
                            ]
                    )
                    args.history
              )
            , ( "message", E.string args.message )
            , ( "handoff", E.bool args.handoff )
            ]
        )
        (D.map2 DemoReply
            (D.field "reply" D.string)
            (D.field "handed_off" D.bool)
        )



-- ACCOUNT


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


type alias WizardView =
    { step : String
    , completed : Bool
    , basics : Basics
    , whatsapp : List WhatsAppAccount
    , signup : Maybe MetaSignup
    , creditsBalance : Int
    , pricing : Pricing
    }


type alias Basics =
    { brandName : String
    , contactName : String
    , contactPhone : String
    , entityType : String
    , pan : String
    , gstin : String
    , address : String
    , state : String
    , pin : String
    }


emptyBasics : Basics
emptyBasics =
    { brandName = ""
    , contactName = ""
    , contactPhone = ""
    , entityType = ""
    , pan = ""
    , gstin = ""
    , address = ""
    , state = ""
    , pin = ""
    }


{-| Meta Embedded Signup parameters. `state` is a one-shot nonce, so a fresh
one arrives with every read.
-}
type alias MetaSignup =
    { appId : String
    , configId : String
    , state : String
    }


getWizard : (Data WizardView -> msg) -> Cmd msg
getWizard =
    get "/api/wizard" wizardDecoder


saveBasics : Basics -> (Data WizardView -> msg) -> Cmd msg
saveBasics b =
    put "/api/wizard/basics"
        (E.object
            [ ( "name", E.string b.brandName )
            , ( "contact_name", E.string b.contactName )
            , ( "phone", E.string b.contactPhone )
            , ( "business_type", E.string b.entityType )
            , ( "pan", E.string b.pan )
            , ( "gstin", E.string b.gstin )
            , ( "address", E.string b.address )
            , ( "state", E.string b.state )
            , ( "pincode", E.string b.pin )
            ]
        )
        wizardDecoder


channelsDone : (Data WizardView -> msg) -> Cmd msg
channelsDone =
    post "/api/wizard/channels/done" (E.object []) wizardDecoder


{-| Step backwards. The server refuses a forward jump.
-}
stepWizard : String -> (Data WizardView -> msg) -> Cmd msg
stepWizard to =
    post "/api/wizard/step" (E.object [ ( "to", E.string to ) ]) wizardDecoder


completeWizard : (Data WizardView -> msg) -> Cmd msg
completeWizard =
    post "/api/wizard/complete" (E.object []) wizardDecoder


wizardDecoder : Decoder WizardView
wizardDecoder =
    D.map7 WizardView
        (D.field "step" D.string)
        (D.field "completed" D.bool)
        (D.oneOf [ D.field "business" basicsDecoder, D.field "basics" basicsDecoder ])
        (D.field "whatsapp" (D.list whatsAppDecoder))
        (D.field "signup" (D.nullable signupDecoder))
        (D.oneOf [ D.at [ "launch", "balance" ] D.int, D.field "credits_balance" D.int ])
        (D.oneOf [ D.field "launch" pricingDecoder, D.field "pricing" pricingDecoder ])


basicsDecoder : Decoder Basics
basicsDecoder =
    D.map8 (\bn cn cp et pan g a s -> Basics bn cn cp et pan g a s "")
        (D.oneOf [ D.field "name" D.string, D.field "brand_name" D.string ])
        (D.field "contact_name" D.string)
        (D.oneOf [ D.field "phone" D.string, D.field "contact_phone" D.string ])
        (D.oneOf [ D.field "business_type" D.string, D.field "entity_type" D.string ])
        (D.field "pan" D.string)
        (D.field "gstin" D.string)
        (D.field "address" D.string)
        (D.field "state" D.string)
        |> D.andThen
            (\b ->
                D.map (\p -> { b | pin = p })
                    (D.oneOf [ D.field "pincode" D.string, D.field "pin" D.string ])
            )


signupDecoder : Decoder MetaSignup
signupDecoder =
    D.map3 MetaSignup
        (D.field "app_id" D.string)
        (D.field "config_id" D.string)
        (D.field "state" D.string)



-- WHATSAPP


type alias WhatsAppAccount =
    { id : String
    , name : String
    , phoneNumberId : String
    , displayPhoneNumber : String
    , reply : ReplyConfig
    }


{-| `mode` is "ai" or "canned". `text` is the static message when `mode` is "canned".
-}
type alias ReplyConfig =
    { enabled : Bool
    , mode : String
    , text : String
    , waitSeconds : Int
    }


getWhatsApp : (Data { accounts : List WhatsAppAccount, signup : Maybe MetaSignup } -> msg) -> Cmd msg
getWhatsApp =
    get "/api/whatsapp"
        (D.map2 (\a s -> { accounts = a, signup = s })
            (D.field "accounts" (D.list whatsAppDecoder))
            (D.field "signup" (D.nullable signupDecoder))
        )


updateWhatsApp :
    String
    -> { name : String, enabled : Bool, mode : String, text : String, waitSeconds : Int }
    -> (Data WhatsAppAccount -> msg)
    -> Cmd msg
updateWhatsApp id args =
    put ("/api/whatsapp/" ++ id)
        (E.object
            [ ( "name", E.string args.name )
            , ( "enabled", E.bool args.enabled )
            , ( "mode", E.string args.mode )
            , ( "text", E.string args.text )
            , ( "wait_seconds", E.int args.waitSeconds )
            ]
        )
        whatsAppDecoder


deleteWhatsApp : String -> (Data () -> msg) -> Cmd msg
deleteWhatsApp id =
    delete ("/api/whatsapp/" ++ id) (E.object []) (D.succeed ())


whatsAppDecoder : Decoder WhatsAppAccount
whatsAppDecoder =
    D.map5 WhatsAppAccount
        (D.field "id" D.string)
        (D.field "name" D.string)
        (D.field "phone_number_id" D.string)
        (D.oneOf [ D.field "display_phone_number" D.string, D.field "phone_number" D.string ])
        (D.field "reply" replyDecoder)


replyDecoder : Decoder ReplyConfig
replyDecoder =
    D.map4 ReplyConfig
        (D.field "enabled" D.bool)
        (D.field "mode" D.string)
        (D.field "text" D.string)
        (D.field "wait_seconds" D.int)



-- BILLING


type alias BillingView =
    { balance : Int
    , repliesUsed : Int
    , metered : Bool
    , currency : String
    , pricing : Pricing
    , ledger : List CreditEntry
    }


type alias CreditEntry =
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


getBilling : (Data BillingView -> msg) -> Cmd msg
getBilling =
    get "/api/billing" billingDecoder


startCheckout : Int -> (Data Order -> msg) -> Cmd msg
startCheckout credits =
    post "/api/billing/checkout"
        (E.object [ ( "credits", E.int credits ) ])
        (orderDecoder credits)


orderDecoder : Int -> Decoder Order
orderDecoder fallbackCredits =
    D.map5 Order
        (D.field "order_id" D.string)
        (D.field "amount" D.int)
        (D.field "currency" D.string)
        (D.field "key_id" D.string)
        (withDefault fallbackCredits (D.field "credits" D.int))


billingDecoder : Decoder BillingView
billingDecoder =
    D.map6 BillingView
        (D.field "balance" D.int)
        (D.field "replies_used" D.int)
        (D.field "metered" D.bool)
        (D.field "currency" D.string)
        (D.field "pricing" pricingDecoder)
        (D.field "ledger" (D.list creditDecoder))


creditDecoder : Decoder CreditEntry
creditDecoder =
    D.map3 CreditEntry
        (D.field "amount" D.int)
        (D.field "source" D.string)
        (withDefault Nothing (D.field "expires_at" (D.nullable D.string)))



-- SETTINGS


type alias SettingsView =
    { conversation : ConversationSettings
    , notifications : NotificationSettings
    , locale : String
    , currency : String
    }


type alias NotificationSettings =
    { handoffEmail : Bool
    }


type alias ConversationSettings =
    { idleGapMins : Maybe Int
    , handoffCooldownMins : Maybe Int
    , maxHistoryMessages : Maybe Int
    , defaults : ConversationBounds
    , min : ConversationBounds
    , max : ConversationBounds
    }


type alias ConversationBounds =
    { idleGapMins : Int
    , handoffCooldownMins : Int
    , maxHistoryMessages : Int
    }


defaultBounds : ConversationBounds
defaultBounds =
    { idleGapMins = 240
    , handoffCooldownMins = 60
    , maxHistoryMessages = 20
    }


defaultMinBounds : ConversationBounds
defaultMinBounds =
    { idleGapMins = 5
    , handoffCooldownMins = 5
    , maxHistoryMessages = 4
    }


defaultMaxBounds : ConversationBounds
defaultMaxBounds =
    { idleGapMins = 1440
    , handoffCooldownMins = 1440
    , maxHistoryMessages = 200
    }


getSettings : (Data SettingsView -> msg) -> Cmd msg
getSettings =
    get "/api/settings" settingsDecoder


saveConversation : ConversationSettings -> (Data SettingsView -> msg) -> Cmd msg
saveConversation conversation =
    put "/api/settings"
        (E.object
            [ ( "conversation"
              , E.object
                    [ ( "idle_gap_mins", maybeInt conversation.idleGapMins )
                    , ( "handoff_cooldown_mins", maybeInt conversation.handoffCooldownMins )
                    , ( "max_history_messages", maybeInt conversation.maxHistoryMessages )
                    ]
              )
            ]
        )
        settingsDecoder


saveNotifications : NotificationSettings -> (Data SettingsView -> msg) -> Cmd msg
saveNotifications notifications =
    put "/api/settings"
        (E.object
            [ ( "notifications"
              , E.object
                    [ ( "handoff_email", E.bool notifications.handoffEmail ) ]
              )
            ]
        )
        settingsDecoder


saveLocale : { locale : String, currency : String } -> (Data SettingsView -> msg) -> Cmd msg
saveLocale fields =
    put "/api/settings"
        (E.object
            [ ( "locale", E.string fields.locale )
            , ( "currency", E.string fields.currency )
            ]
        )
        settingsDecoder


maybeInt : Maybe Int -> E.Value
maybeInt =
    Maybe.map E.int >> Maybe.withDefault E.null


settingsDecoder : Decoder SettingsView
settingsDecoder =
    D.map4 SettingsView
        (D.field "conversation" conversationDecoder)
        (D.field "notifications" notificationsDecoder)
        (D.field "locale" D.string)
        (D.field "currency" D.string)


notificationsDecoder : Decoder NotificationSettings
notificationsDecoder =
    D.map NotificationSettings
        (D.field "handoff_email" D.bool)


conversationDecoder : Decoder ConversationSettings
conversationDecoder =
    D.map6 ConversationSettings
        (D.field "idle_gap_mins" (D.nullable D.int))
        (D.field "handoff_cooldown_mins" (D.nullable D.int))
        (D.field "max_history_messages" (D.nullable D.int))
        (withDefault defaultBounds (D.field "defaults" boundsDecoder))
        (withDefault defaultMinBounds (D.field "min" boundsDecoder))
        (withDefault defaultMaxBounds (D.field "max" boundsDecoder))


boundsDecoder : Decoder ConversationBounds
boundsDecoder =
    D.map3 ConversationBounds
        (D.field "idle_gap_mins" D.int)
        (D.field "handoff_cooldown_mins" D.int)
        (D.field "max_history_messages" D.int)



-- HELPERS


withDefault : a -> Decoder a -> Decoder a
withDefault fallback decoder =
    D.oneOf [ decoder, D.succeed fallback ]
