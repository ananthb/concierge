port module Ports exposing
    ( MetaSignupOutcome(..)
    , PaymentOutcome
    , decodeMetaSignupOutcome
    , metaSignupOutcome
    , openCheckout
    , openMetaSignup
    , paymentOutcome
    )

{-| The only JavaScript this app talks to.

Razorpay's checkout and Meta's Embedded Signup are hosted modals opened by
their own SDKs — there is no redirect-only flow to use instead, so they use
ports. The glue lives in `public/boot.js`.

Everything else the app needs comes from `/api/*`. Notably **not** here:

  - Money formatting. `Format` does it in pure Elm, because `view` can't wait
    for a port round-trip.
  - The session. It's an `HttpOnly` cookie the browser attaches itself, so
    there is nothing for JavaScript to read or hand over.

-}

import Json.Decode as D
import Json.Encode as E


{-| Hand Razorpay an order and let it take over the screen.

The worker created the order; `key_id` is the publishable key. Success or
failure comes back through [`paymentOutcome`].

-}
port openCheckout : E.Value -> Cmd msg


{-| What Razorpay reported.

`signature` is empty on a dismissal or failure. Credits are granted by the
Razorpay webhook (`POST /api/billing/webhook`).

-}
type alias PaymentOutcome =
    { orderId : String
    , paymentId : String
    , signature : String
    , error : String
    }


port paymentOutcome : (PaymentOutcome -> msg) -> Sub msg


{-| Launch Meta's Embedded Signup JS SDK popup for WhatsApp Business.

Success, cancellation, or error comes back through [`metaSignupOutcome`].

-}
port openMetaSignup : { appId : String, configId : String, state : String } -> Cmd msg


port metaSignupOutcome : (E.Value -> msg) -> Sub msg


type MetaSignupOutcome
    = MetaSignupOk
    | MetaSignupCancelled
    | MetaSignupError String


decodeMetaSignupOutcome : E.Value -> MetaSignupOutcome
decodeMetaSignupOutcome value =
    case D.decodeValue outcomeDecoder value of
        Ok outcome ->
            outcome

        Err _ ->
            MetaSignupError "Could not read WhatsApp signup result."


outcomeDecoder : D.Decoder MetaSignupOutcome
outcomeDecoder =
    D.field "status" D.string
        |> D.andThen
            (\status ->
                case status of
                    "ok" ->
                        D.succeed MetaSignupOk

                    "cancelled" ->
                        D.succeed MetaSignupCancelled

                    _ ->
                        D.oneOf
                            [ D.field "message" D.string |> D.map MetaSignupError
                            , D.succeed (MetaSignupError "WhatsApp signup failed. Please try again.")
                            ]
            )
