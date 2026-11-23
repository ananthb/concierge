module Page.Static exposing (features, login, notFound, pricing, privacy, terms)

{-| Pages with no state of their own: marketing copy, the legal text, the
sign-in screen, and the 404.
-}

import Api
import Dict
import Format
import Html exposing (Html, a, div, h1, h2, h3, li, p, section, text, ul)
import Html.Attributes exposing (class, href)
import Route
import Ui


pricing : Api.Data Api.Pricing -> Html msg
pricing rates =
    div [ class "page page-pricing" ]
        [ h1 [] [ text "Pay per reply, nothing else" ]
        , p [ class "page-sub" ]
            [ text "Prepaid credits only — 1 credit = 1 reply (static or AI), no subscriptions, no free trials, top up anytime with Razorpay." ]
        , section [ class "price-cards" ]
            [ Ui.remote rates <|
                \p_ ->
                    div [ class "price-cards__inner" ]
                        (List.map (rateCard p_) (Dict.toList p_.unitPriceMilli))
            ]
        , section [ class "price-notes" ]
            [ h2 [] [ text "How credits work" ]
            , ul []
                [ li [] [ text "Prepaid credits only — 1 credit = 1 WhatsApp reply (static or AI)." ]
                , li [] [ text "No subscriptions, no per-seat fees, and no free trials." ]
                , li [] [ text "A message replybots holds back — or if AI generation fails — is not charged." ]
                , li [] [ text "Purchased credits do not expire." ]
                ]
            , h2 [] [ text "Topping up" ]
            , Ui.remote rates <|
                \p_ ->
                    p []
                        [ text
                            ("Top up anytime in your dashboard with Razorpay, starting from "
                                ++ Format.count "en-IN" p_.minCredits
                                ++ " credits up to "
                                ++ Format.count "en-IN" p_.maxCredits
                                ++ " credits per order."
                            )
                        ]
            ]
        , div [ class "page-cta" ]
            [ a [ href "/auth/login", class "btn btn-primary btn-lg" ] [ text "Get started" ] ]
        ]


rateCard : Api.Pricing -> ( String, Int ) -> Html msg
rateCard _ ( code, unitPriceMilli ) =
    div [ class "price-card" ]
        [ h3 [] [ text code ]
        , p [ class "price" ]
            [ text (Format.milliRate "en-IN" code unitPriceMilli) ]
        , p [ class "muted" ] [ text "per reply (1 credit = 1 reply)" ]
        ]


features : Html msg
features =
    div [ class "page page-features" ]
        [ h1 [] [ text "Built for small businesses on WhatsApp" ]
        , section [ class "feature-list" ]
            [ feature "Static or AI quick-handoff replies"
                "Choose a fixed static message or let replybots's built-in front-desk assistant answer briefly on your WhatsApp Business number."
            , feature "Fast human handoff + email alert"
                "The moment a customer asks for a quote, a commitment, or personal attention, replybots hands off cleanly and optionally emails you so you can step in."
            , feature "Burst message batching"
                "Wait 0 to 30 seconds for customers who send multiple short messages in a row, so they get one coherent reply instead of three."
            , feature "Conversation session window"
                "Configure the idle gap, handoff cooldown, and recent message context so threads stay relevant without repeating stale history."
            , feature "Cloudflare AI safety & guardrails"
                "Every reply passes through Cloudflare AI safety and prompt-injection guardrails designed to stop off-topic drift and unauthorized promises."
            , feature "Metadata-only privacy"
                "Customer message text passes through in memory to produce a reply and is never stored in our database — only channel, direction, sender, recipient, and timestamp are kept."
            , feature "Self-service Razorpay prepaid credits"
                "Prepaid credits only (1 credit = 1 reply). Buy what you need when you need it with Razorpay — no subscriptions or lock-in."
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
        [ h1 [] [ text "Sign in to replybots" ]
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
          , [ "replybots connects to your WhatsApp Business Account through Meta Embedded Signup to send automated replies on your behalf. You keep ownership of your phone number and WhatsApp Business Account; we process messages only on your instructions."
            , "Automated replies are either static messages you write or responses generated by replybots's built-in front-desk AI assistant. We do not guarantee that any AI-generated reply is accurate, complete, or suitable for your business. You remain responsible for all communications sent from your WhatsApp Business number."
            ]
          )
        , ( "Acceptable use"
          , [ "Do not use replybots to send spam, unsolicited bulk messages, deceptive communications, or anything unlawful or prohibited by Meta's WhatsApp Business Messaging Policy."
            , "We may suspend or terminate an account that violates these terms or that Meta requires us to restrict."
            ]
          )
        , ( "Prepaid credits and payment"
          , [ "replybots operates strictly on prepaid credits: 1 credit = 1 reply (whether static or AI). There are no subscriptions and no free trials."
            , "You can purchase credits anytime through Razorpay in your dashboard. A reply that fails before delivery or is withheld before sending is not charged. Purchased credits do not expire."
            ]
          )
        , ( "Availability"
          , [ "We aim to keep replybots reliable, but offer no uptime SLA. Meta's WhatsApp Cloud API, Cloudflare Workers/AI, and Razorpay are outside our control, and an outage in any dependency will interrupt service."
            ]
          )
        , ( "Liability"
          , [ "To the extent permitted by law, our total liability to you is limited to the amount you paid us in the three months preceding the claim. We are not liable for indirect or consequential losses, lost revenue, or lost business."
            ]
          )
        , ( "Ending your account"
          , [ "You may stop using replybots and permanently delete your account at any time from Dashboard Settings. Unused prepaid credits are non-refundable except where required by law."
            ]
          )
        , ( "Contact"
          , [ "Questions about these terms: support@replybots.io."
            ]
          )
        ]


privacy : Html msg
privacy =
    legalPage "Privacy Policy"
        [ ( "Metadata-only message logging"
          , [ "We do not store the text content of your customers' WhatsApp messages in our database. Inbound and outbound message bodies pass through in memory to generate or dispatch a reply and are not persisted."
            ]
          )
        , ( "What we store"
          , [ "Message metadata: channel, direction (inbound or outbound), sender and recipient identifiers, timestamp, and handling outcome (replied or handed off) — never the message body."
            , "Account records: your Google account email and name, business and invoicing details you enter during setup, your WhatsApp Business connection metadata (phone number ID and display number), notification preferences, and your prepaid credit balance and Razorpay payment records."
            ]
          )
        , ( "Third-party processors"
          , [ "Meta / WhatsApp: delivers and receives your WhatsApp Business messages and powers Meta Embedded Signup."
            , "Cloudflare: hosts the application and runs Workers AI inference and safety guardrails. Message text sent for reply generation is not used to train models."
            , "Razorpay: processes prepaid credit payments. We never receive or store your card or bank details."
            , "Google: authenticates your sign-in and provides your email address and display name."
            ]
          )
        , ( "Retention and deletion"
          , [ "Account records, WhatsApp connection details, and message metadata are kept only while your account remains active."
            , "You can permanently delete your account and associated metadata yourself at any time in /dashboard/settings. Payment records required by tax and accounting law are retained with your account identifier removed."
            , "We also expose Meta's required /data-deletion callback endpoint for WhatsApp Business data-deletion requests."
            ]
          )
        , ( "Contact"
          , [ "Privacy inquiries and deletion requests: support@replybots.io."
            ]
          )
        ]


legalPage : String -> List ( String, List String ) -> Html msg
legalPage title sections =
    div [ class "page page-legal" ]
        [ h1 [] [ text title ]
        , p [ class "muted" ] [ text "Last updated 22 November 2026." ]
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
