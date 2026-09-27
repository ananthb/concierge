module Page.Static exposing (features, login, notFound, pricing, privacy, terms)

{-| Pages with no state of their own: marketing copy, the legal text, the
sign-in screen, and the 404.

The legal text is held here as Elm rather than fetched, because it has to be
readable whether or not the API is up, and because a terms page that can fail
to load is a liability. It was `src/legal.rs` before.

These are also the pages that suffer most from there being no server-side
rendering: a crawler sees an empty shell. The copy is here, in one place, for
whatever prerender step comes next to pick up.

-}

import Api
import Format
import Html exposing (Html, a, div, h1, h2, h3, li, p, section, span, text, ul)
import Html.Attributes exposing (class, href)
import Route
import Ui


{-| The pricing page.

Takes the rates as `Data` rather than a resolved value so the copy renders on
first paint and only the numbers wait. That is what makes a prerendered
snapshot of this page useful: a crawler gets the prose, and the rates are
never baked into a static file where they could go stale against the
operator-configured values.

-}
pricing : Api.Data Api.Pricing -> Html msg
pricing rates =
    div [ class "page page-pricing" ]
        [ h1 [] [ text "Pay for replies, nothing else" ]
        , p [ class "page-sub" ]
            [ text "No subscription, no per-seat fee, no minimum. Buy a pile of replies and use them whenever." ]
        , section [ class "price-cards" ]
            [ Ui.remote rates <|
                \p_ -> div [ class "price-cards__inner" ] (List.map (rateCard p_) p_.currencies)
            ]
        , section [ class "price-notes" ]
            [ h2 [] [ text "What counts as a reply" ]
            , ul []
                [ li [] [ text "An AI-written answer costs one reply." ]
                , li [] [ text "A fixed message you wrote costs nothing, however many go out." ]
                , li [] [ text "A message Concierge decides not to answer — because it's about money, or it's been handed to you — costs nothing." ]
                , li [] [ text "If the AI fails mid-reply, the credit goes back." ]
                ]
            , h2 [] [ text "Buying" ]
            , p []
                [ text "Any quantity, upwards of the minimum shown above. Credits you buy don't expire." ]
            ]
        , div [ class "page-cta" ]
            [ a [ href "/auth/login", class "btn btn-primary btn-lg" ] [ text "Get started" ] ]
        ]


rateCard : Api.Pricing -> { code : String, unitPriceMilli : Int } -> Html msg
rateCard _ rate =
    div [ class "price-card" ]
        [ h3 [] [ text rate.code ]
        , p [ class "price" ]
            [ text (Format.milliRate "en-IN" rate.code rate.unitPriceMilli) ]
        , p [ class "muted" ] [ text "per AI reply" ]
        ]


features : Html msg
features =
    div [ class "page page-features" ]
        [ h1 [] [ text "What Concierge does" ]
        , section [ class "feature-list" ]
            [ feature "Answers WhatsApp for you"
                "Messages to your business number get a reply in your own voice, within seconds, at any hour. Your number and your customers' threads stay exactly where they are."
            , feature "Knows when to stop"
                "Anything that quotes a price, promises a date or strays outside what you said it should discuss is never sent. Concierge tells the customer a person is picking it up, and emails you."
            , feature "Waits for people to finish typing"
                "A burst of three short messages gets one considered answer, not three. You choose how long it waits."
            , feature "Remembers the conversation"
                "Replies use the thread's recent history. After a few hours of silence the conversation is over and the next message starts fresh."
            , feature "A prompt you can actually read"
                "Pick a voice, answer a few questions, and Concierge writes the prompt. The fixed guardrails around it are visible on the page — nothing about what the model is told is hidden from you."
            , feature "Checked before it speaks"
                "Every prompt goes past a safety classifier before it's allowed to answer a customer. Change the prompt and AI replies pause until the new one clears."
            , feature "Keeps almost nothing"
                "Message bodies pass through and are never written down. Only channel, direction, sender, recipient and timestamp are stored."
            ]
        , div [ class "page-cta" ]
            [ a [ href "/auth/login", class "btn btn-primary btn-lg" ] [ text "Get started" ] ]
        ]


feature : String -> String -> Html msg
feature title body =
    div [ class "feature" ]
        [ h2 [] [ text title ]
        , p [] [ text body ]
        ]


login : Html msg
login =
    div [ class "page page-login" ]
        [ h1 [] [ text "Sign in to Concierge" ]
        , p [ class "page-sub" ]
            [ text "We use your Google account. There's no password to remember." ]
        , a [ href "/auth/login", class "btn btn-primary btn-lg" ] [ text "Continue with Google" ]
        , p [ class "fine-print" ]
            [ text "By signing in you agree to our "
            , a [ Route.href Route.Terms ] [ text "terms" ]
            , text " and "
            , a [ Route.href Route.Privacy ] [ text "privacy policy" ]
            , text "."
            ]
        ]


notFound : Html msg
notFound =
    div [ class "page page-404" ]
        [ h1 [] [ text "Nothing here" ]
        , p [] [ text "That page doesn't exist." ]
        , a [ Route.href Route.Landing, class "btn btn-secondary" ] [ text "Back to the start" ]
        ]


terms : Html msg
terms =
    legalPage "Terms of Service"
        [ ( "The service"
          , [ "Concierge replies to messages sent to a WhatsApp business number you connect. You keep the number and the WhatsApp Business account; we act on your instructions."
            , "Replies are generated by a language model working from a prompt you configure. We do not guarantee that any particular reply is accurate, appropriate or complete. You are responsible for what your business says to its customers, including what Concierge says on your behalf."
            ]
          )
        , ( "What you must not do"
          , [ "Do not use Concierge to send unsolicited bulk messages, to impersonate anyone, or for anything unlawful. Do not configure a prompt intended to deceive customers about whether they are talking to an automated system."
            , "We may suspend an account that breaks these rules, or that Meta requires us to suspend."
            ]
          )
        , ( "Replies and payment"
          , [ "AI replies are prepaid. A fixed message you wrote costs nothing. A reply that fails, or one Concierge withholds, is not charged."
            , "Credits you buy do not expire. Credits we grant may carry an expiry, which is shown on your billing page."
            , "At sign-up we charge a small amount to verify your payment method and refund it immediately."
            ]
          )
        , ( "Availability"
          , [ "We aim to keep Concierge running but offer no uptime guarantee. Meta's WhatsApp API, our AI provider and our hosting are all outside our control, and an outage in any of them will stop replies going out."
            ]
          )
        , ( "Liability"
          , [ "To the extent the law allows, our liability to you is limited to the amount you paid us in the three months before the claim. We are not liable for lost business, lost customers or lost data."
            ]
          )
        , ( "Ending it"
          , [ "You can stop using Concierge at any time and ask us to delete your account. Unused credits are not refundable except where the law requires it. We may end your account with reasonable notice, or immediately if you break these terms."
            ]
          )
        , ( "Changes"
          , [ "We may change these terms. Continuing to use Concierge after a change means you accept it."
            ]
          )
        ]


privacy : Html msg
privacy =
    legalPage "Privacy Policy"
        [ ( "The short version"
          , [ "We do not store the contents of your customers' messages. They pass through our worker, are sent to an AI model to generate a reply, and are not written to any database we keep."
            ]
          )
        , ( "What we store"
          , [ "For every message: the channel, whether it was inbound or outbound, the sender, the recipient, a timestamp, and what Concierge did with it. Not the text."
            , "For your account: your email address and name from Google, your business details as you entered them, your prompt configuration, and your credit balance and payment records."
            ]
          )
        , ( "Who else sees it"
          , [ "Meta, because that is where WhatsApp messages come from and go to."
            , "Cloudflare, who host the service and run the AI model that writes replies. Message text is sent to that model to produce a reply and is not used to train it."
            , "Razorpay, for payments. We never see or store your card details."
            , "Google, if you sign in with them — they tell us your email address and name."
            ]
          )
        , ( "How long"
          , [ "Message metadata and account records are kept while your account is open. Payment records are kept as long as tax and accounting law requires, which is longer."
            ]
          )
        , ( "Your data"
          , [ "You can ask us to delete your account and the metadata attached to it. Payment records survive with your account identifier removed."
            , "There is also a data-deletion endpoint at /data-deletion, which exists because Meta requires one."
            ]
          )
        , ( "Contact"
          , [ "Questions about any of this: support@calculon.tech."
            ]
          )
        ]


legalPage : String -> List ( String, List String ) -> Html msg
legalPage title sections =
    div [ class "page page-legal" ]
        [ h1 [] [ text title ]
        , p [ class "muted" ] [ text "Last updated 26 September 2026." ]
        , div []
            (List.map
                (\( heading, paragraphs ) ->
                    section []
                        (h2 [] [ text heading ]
                            :: List.map (\body -> p [] [ text body ]) paragraphs
                        )
                )
                sections
            )
        ]
