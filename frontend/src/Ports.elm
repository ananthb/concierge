port module Ports exposing (PaymentOutcome, openCheckout, paymentOutcome)

{-| The only JavaScript this app talks to.

Razorpay's checkout is a hosted modal opened by their own SDK — there is no
redirect-only flow to use instead, so it has to be a port. The glue lives in
the shell (`src/shell.rs`).

Everything else the app needs comes from `/api/*`. Notably **not** here:

  - Money formatting. `Format` does it in pure Elm, because `view` can't wait
    for a port round-trip.
  - The session. It's an `HttpOnly` cookie the browser attaches itself, so
    there is nothing for JavaScript to read or hand over.

-}

import Json.Encode as E


{-| Hand Razorpay an order and let it take over the screen.

The worker created the order; `keyId` is the publishable key. Success or
failure comes back through [`paymentOutcome`].

-}
port openCheckout : E.Value -> Cmd msg


{-| What Razorpay reported.

`signature` is empty on a dismissal or failure. A success still has to be
confirmed server-side (`POST /api/billing/verify`) — credits are granted by
the Razorpay webhook, so nothing here can mint them.

-}
type alias PaymentOutcome =
    { orderId : String
    , paymentId : String
    , signature : String
    , error : String
    }


port paymentOutcome : (PaymentOutcome -> msg) -> Sub msg
