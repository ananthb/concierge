# Concierge UI strings — en-IN (India / Indian English).
#
# This is the canonical source of truth for translatable copy. Any new
# string in templates/handlers MUST appear here first; the build script
# refuses to compile if a `t!("key")` reference is missing from this file.
#
# Other locales' FTL files (e.g. en-US, hi-IN) override individual messages
# but inherit any not-overridden ones from this bundle. Keep keys grouped
# by page / feature.

# Public navigation (top of every public page).
nav-features = Features
nav-pricing = Pricing
nav-open-source = Open source ↑
nav-sign-in = Sign in

# Shared site footer.
footer-features = Features
footer-pricing = Pricing
footer-docs = Docs
footer-open-source = Open-source
footer-licence = AGPL-3.0
footer-terms = Terms of Service
footer-privacy = Privacy Policy

# Admin app shell (top nav inside /admin).
app-nav-overview = Overview
app-nav-approvals = Approvals
app-nav-channels = Channels
app-nav-email = Email Routing
app-nav-billing = Billing
app-nav-settings = Settings
app-nav-aria-label = Admin sections
app-nav-skip-link = Skip to main content
app-nav-status-live = live
app-nav-logout-aria = Sign out

# Maintenance fallback page.
maintenance-title = Concierge is offline
maintenance-headline = Be right back.
maintenance-body = Concierge is briefly unavailable while we finish a configuration change. We'll be back in a few minutes: thanks for your patience.
maintenance-tail = If this persists, please check back shortly.

# Default OpenGraph / meta description for pages that don't override.
meta-default-description = Automated customer messaging for small businesses. Auto-reply across WhatsApp, Instagram DMs, and email. 100 AI replies included every month.

# Inline JS-driven UI strings (rendered into data-* attributes server-side
# so the script can read them without re-fetching the bundle).
js-copy-button-default = Copy
js-copy-button-copied = Copied!
js-htmx-error-toast = Something went wrong. Please try again.

# Welcome / landing page (/).
welcome-eyebrow = // automated customer engagement
welcome-headline = Hello. I'll be answering <br>every <em>DM, WhatsApp &amp; email</em> <br>so you don't have to.
# Alternative headlines cycled in via JS on the welcome page (typewriter
# rotation). Keep the same three-line shape and channel-list emphasis so the
# hero block stays visually stable while the wording varies.
welcome-headline-2 = Hi. I'll write back to <br>every <em>DM, WhatsApp &amp; email</em> <br>before they go cold.
welcome-headline-3 = Hello. I'll handle <br>every <em>DM, WhatsApp &amp; email</em> <br>while you run the shop.
welcome-headline-4 = Hi. I'm on <br>every <em>DM, WhatsApp &amp; email</em>, <br>day, night, and weekends.
welcome-headline-5 = Hello. I'll cover <br>every <em>DM, WhatsApp &amp; email</em> <br>so nobody waits on you.
welcome-lead = Concierge is an automated customer-messaging service for small businesses. Connect your channels, set a tone, and I'll auto-reply across WhatsApp, Instagram, Discord, and email. 100 AI replies included every month.
welcome-cta-primary = Get started →
welcome-cta-secondary = See features

# Live demo chat ("ask me about Concierge"). Triggered from the welcome
# CTA row; backed by /demo/chat.
demo-chat-hint = click to chat with me — try the live demo
demo-chat-title = Live demo
demo-chat-subtitle = Pick a sample business, then message it as if you were one of its customers. You'll see exactly the kind of reply Concierge would send for you.
demo-chat-subtitle-concierge = Ask me anything about Concierge — what I do, channels, pricing, setup.
demo-chat-persona-label = Sample business
demo-chat-roleplay-prefix = You're chatting as a customer of
demo-chat-roleplay-suffix = . The reply you get is what Concierge would send on this business's behalf.
demo-chat-channels-note = In real use, your customers reach you on WhatsApp, Instagram DMs, Discord, or email — not in a chat window like this. This window is just the live demo.
demo-chat-business-hours = Hours
demo-chat-business-city = City
demo-chat-business-type = Type
demo-chat-business-goal = Goal
demo-chat-handoff-chip = A teammate has been notified. Concierge is holding the conversation until a human takes over.
demo-chat-view-prompt = View system prompt
demo-chat-hide-prompt = Hide system prompt
demo-chat-prompt-heading = System prompt sent to the model
demo-chat-envelope-note = The shaded sections are Concierge's safety wrapper — added to every prompt and not editable. The middle section is what the persona supplies, plus (for sample businesses) a one-time demo frame that asks the AI to nudge you to sign up at the end of the conversation.
demo-chat-placeholder = Ask me anything…
demo-chat-placeholder-customer-prefix = Message
demo-chat-placeholder-customer-suffix = as a customer…
demo-chat-send = Send
demo-chat-close = Close
demo-chat-thinking = Thinking…
demo-chat-error = Something went wrong. Try again in a moment.
demo-chat-rate-limited = You've sent quite a few messages — give me a minute and try again.

# Features page (/features).
features-meta-description = Concierge auto-replies on WhatsApp, Instagram, Discord, and email. AI replies { $inr } / { $usd }, 100 included per month; static replies don't consume credits. 5-minute setup. Open source.
features-og-title = Concierge Features
features-headline = One assistant. Every channel.
features-lead = Concierge replies for you on WhatsApp, Instagram, Discord, and email: instantly, in your voice, and only when it should.
features-channels-heading = Channels
features-card-whatsapp-eyebrow = WhatsApp Business
features-card-whatsapp-body = Connect a number, pick a tone, ship. Customer messages get an instant static or AI reply.
features-card-instagram-eyebrow = Instagram DMs
features-card-instagram-body = Sign in with Meta, choose the business account. Replies go out through the official Graph API.
features-card-discord-eyebrow = Discord
features-card-discord-body = Install the bot. The concierge replies when @-mentioned or in channels you designate.
features-card-email-eyebrow = Email
features-card-email-body = Pick a name at <code>name@cncg.email</code>. Inbound mail gets a reply; you and your team get a copy via Cc/Bcc.
features-how-heading = How it works
features-card-step-1-eyebrow = 1. Connect
features-card-step-1-body = 5-minute wizard. Authenticate with your WhatsApp for Business, Instagram Pages, and Discord Channels.
features-card-step-2-eyebrow = 2. Configure
features-card-step-2-body = Pick static or AI replies per channel. Set a persona: tone, business type, things to never say.
features-card-step-3-eyebrow = 3. Run
features-card-step-3-body = Concierge replies to incoming messages for you in the channels you use. Pick up the conversation when you're ready!
features-trust-heading = AI you can trust
features-card-voice-eyebrow = Voice and guardrails
features-card-voice-body = Set tone, biz type, and things the AI must never do. Every reply respects those rules.
features-card-injection-eyebrow = Prompt-injection screening
features-card-injection-body = Inbound text is scanned before it reaches the model. Suspicious messages are skipped, not auto-replied.
features-card-pay-eyebrow = Pay only for AI
features-card-pay-body = AI replies cost { $inr } / { $usd } each, with 100 included per month. Static auto-replies you author yourself don't consume credits.
features-more-heading = More that just works
features-card-leads-eyebrow = Lead capture
features-card-leads-body = Embed a phone-number form on any page. Submissions trigger an instant WhatsApp message.
features-card-recipients-eyebrow = Notification recipients
features-card-recipients-body = Add Cc/Bcc emails to any concierge address. We verify each one with a one-click link.
features-card-privacy-eyebrow = Privacy by default
features-card-privacy-body = We log metadata, never message bodies. Account deletion wipes everything, immediately.
features-card-os-eyebrow = Open source
features-card-os-body = AGPL-3.0 on <a href="https://github.com/ananthb/concierge" target="_blank" rel="noopener">GitHub</a>. Self-host if you'd rather. <a href="https://ananthb.github.io/concierge/" target="_blank" rel="noopener">Architecture docs</a>.
features-cta-heading = Ready to set this up?
features-cta-body = Sign in with Google or Facebook. The wizard takes 5 minutes.
features-cta-primary = Get started →
features-cta-secondary = See pricing

# Pricing page (/pricing).
pricing-meta-description = Simple, prepaid pricing for Concierge. { $inr } / { $usd } per AI reply, no tiers. 100 AI replies included every month. Reply-email subscription: { $pack_size } addresses for { $addr_inr } / { $addr_usd } per month.
pricing-og-title = Concierge Pricing
pricing-headline-prefix = per AI reply. Static auto-replies don't consume credits.
pricing-currency-inr-label = Indian rupees
pricing-currency-usd-label = US dollars
pricing-lead = 100 AI replies included with every account every month. After that, top up with as many credits as you want: no tiers, no contracts.
pricing-credits-eyebrow = What costs a credit?
pricing-credits-li-1 = <strong>AI auto-replies</strong> on WhatsApp, Instagram, email, or Discord: <strong>1 credit each.</strong>
pricing-credits-li-2 = <strong>Static auto-replies</strong> (canned text you wrote yourself): no credits consumed.
pricing-credits-li-3 = Inbound messages, notification CCs/BCCs, Discord relay, slash commands: no credits consumed.
pricing-email-heading = Reply-email subscription
pricing-email-body = Each address you set up at <code>name@cncg.email</code> can auto-reply to inbound mail. Replies go to the original sender; you and your team get a copy via Cc/Bcc.
pricing-email-quota-prefix = <strong>{ $pack_size } addresses per pack</strong>, billed
pricing-email-quota-suffix = per pack, monthly. Cancel any time.
pricing-email-billing-note = AI replies draw from your credit balance above. Static replies don't consume credits.
pricing-slider-cta-signin = Sign in to buy

# Onboarding wizard chrome.
wizard-title = Concierge - Setup
wizard-step-basics-label = The basics
wizard-step-channels-label = Plug in
wizard-step-notifications-label = Heads up
wizard-step-replies-label = Replies
wizard-step-launch-label = Ship it
wizard-signout = Sign out
wizard-back = ← Back
wizard-continue = Continue →

# Wizard step 1: The basics.
wizard-basics-eyebrow = The basics
wizard-basics-headline = Tell us about you.
wizard-basics-lead = For invoicing and compliance. Your details are never shared.
wizard-basics-label-name = Brand name *
wizard-basics-label-contact = Your name *
wizard-basics-label-phone = Phone *
wizard-basics-label-type = Entity type
wizard-basics-label-pan = PAN
wizard-basics-label-gstin-prefix = GSTIN
wizard-basics-label-gstin-suffix = (optional)
wizard-basics-label-address = Registered address
wizard-basics-label-state = State
wizard-basics-label-pincode = Pincode
wizard-basics-placeholder-name = Blossom Florist
wizard-basics-placeholder-contact = Full name
wizard-basics-placeholder-phone = +91 98765 43210
wizard-basics-placeholder-pan = ABCDE1234F
wizard-basics-placeholder-gstin = 22AAAAA0000A1Z5
wizard-basics-placeholder-address = Shop 12, Main Road...
wizard-basics-placeholder-state = Tamil Nadu
wizard-basics-placeholder-pincode = 600001
wizard-basics-type-default = Select type...
wizard-basics-type-unregistered = Unregistered / Individual
wizard-basics-type-sole-prop = Sole Proprietorship
wizard-basics-type-partnership = Partnership
wizard-basics-type-pvt-ltd = Private Limited
wizard-basics-type-llp = LLP

# Wizard step 5: Launch (everything else uses inline strings — tracked as
# follow-up after first translator handoff).
wizard-launch-eyebrow = Ship it
wizard-launch-headline = You're all set.
wizard-launch-lead = Your concierge is configured and ready to handle messages. Here's a summary of what's next.
wizard-launch-status-headline = Ready to go live
wizard-launch-status-body = Hit finish to open your dashboard. Connect channels, set up email rules, and start receiving auto-replies.
wizard-launch-finish = Finish setup →
wizard-launch-email-eyebrow = Email addresses
wizard-launch-email-body = These addresses are live. Inbound mail will be auto-replied if you turn on auto-reply for them in Email.
wizard-launch-credits-note = Optional: 100 AI replies are included every month. Top up later from Billing if you need.
wizard-launch-verify-headline = Verify your account
wizard-launch-verify-body = We charge a small refundable amount to confirm a real card. This keeps the platform free of abuse and is required to finish setup.
wizard-launch-verify-cta = Verify with Razorpay
wizard-launch-verify-refund = refunded automatically

# Admin: shared bits.
admin-back = ← Back
admin-save = Save
admin-cancel = Cancel
admin-delete = Delete
admin-remove = Remove
admin-edit = Edit
admin-yes = Yes
admin-no = No
admin-active = Active
admin-disabled = Disabled

# Admin: login screen.
admin-login-tagline = Sign in to manage your messaging channels.
admin-login-google = Continue with Google
admin-login-facebook-continue = Continue with Facebook
admin-login-whatsapp = Continue with WhatsApp
admin-login-whatsapp-connecting = Opening WhatsApp...
admin-login-whatsapp-error = WhatsApp sign-up was cancelled. Please try again.
admin-login-back = ← Back
admin-login-title = Sign In - Concierge

# Admin: settings page.
admin-settings-title = Settings - Concierge
admin-settings-h1 = Settings
admin-settings-linked-h2 = Linked Accounts
admin-settings-linked-lead = Sign-in providers connected to your account.
admin-settings-linked-region = Linked accounts
admin-settings-th-provider = Provider
admin-settings-th-details = Details
admin-settings-currency-h2 = Billing Currency
admin-settings-currency-lead = All future charges will be in this currency.
admin-settings-currency-inr = ₹ INR (Indian Rupee)
admin-settings-currency-usd = $ USD (US Dollar)
admin-settings-session-h2 = Session
admin-settings-signout = Sign Out
admin-settings-delete-h2 = Delete Account
admin-settings-delete-lead = Permanently delete your account and all associated data. This cannot be undone.
admin-settings-delete-cta = Delete My Account
admin-settings-delete-confirm = Are you sure? This will permanently delete your account and ALL data. This cannot be undone.

# Admin: dashboard overview.
admin-dashboard-title = Dashboard - Concierge
admin-side-channels = Connected channels
admin-side-empty-prefix = No channels connected yet.
admin-side-empty-link = Add one
admin-side-email-row-name = Email Routing
admin-side-email-row-cta = Configure rules
admin-side-quick-links = Quick links
admin-side-lead-forms-prefix = Lead Forms
admin-side-email-log = Email Log
admin-dashboard-eyebrow = Overview
admin-dashboard-headline = Your concierge is on duty.
admin-dashboard-stat-whatsapp = WhatsApp
admin-dashboard-stat-instagram = Instagram
admin-dashboard-stat-leads = Lead Forms
admin-dashboard-stat-credits = Reply Credits
admin-dashboard-email-eyebrow = Email Routing
admin-dashboard-email-headline-active = Rules for the mail that comes in.
admin-dashboard-email-headline-empty = Route your business email with AI.
admin-dashboard-email-cta-active = Manage rules
admin-dashboard-email-cta-empty = Set up email →
admin-dashboard-email-desc-active = Configure domains and routing rules to forward, drop, or AI-reply to incoming email.
admin-dashboard-email-desc-empty = Add a domain to auto-forward, AI-reply, or relay emails to Discord. Takes under a minute.
admin-dashboard-risk-banner-headline = AI replies now pause for review when needed.
admin-dashboard-risk-banner-body = If an AI draft mentions money or makes a commitment, we'll queue it for your approval before sending. You can change this per rule.
admin-dashboard-risk-banner-dismiss = Got it

# Admin: WhatsApp + Instagram accessible labels.
admin-icon-instagram = Instagram
admin-icon-whatsapp = WhatsApp
admin-icon-email = Email

# Admin: WhatsApp account list.
admin-wa-list-title = WhatsApp Accounts - Concierge
admin-wa-list-h1 = WhatsApp Accounts
admin-wa-list-add = + Connect WhatsApp Number
admin-wa-list-th-name = Name
admin-wa-list-th-phone = Phone
admin-wa-list-th-auto = Auto-Reply
admin-wa-list-empty = No WhatsApp accounts configured.
admin-wa-list-delete-confirm = Delete this WhatsApp account?

# Admin: WhatsApp signup screen.
admin-wa-signup-title = Connect WhatsApp - Concierge
admin-wa-signup-h1 = Connect WhatsApp Number
admin-wa-signup-lead = Click the button below to register your phone number through Meta's WhatsApp setup flow.
admin-wa-signup-cta = Connect WhatsApp Number
admin-wa-signup-connecting = Connecting...
admin-wa-signup-cancel-error = Signup was cancelled or failed. Please try again.
admin-wa-signup-manual-prefix = Or
admin-wa-signup-manual-link = enter phone number ID manually

# Admin: WhatsApp edit form.
admin-wa-edit-title = Edit WhatsApp Account - Concierge
admin-wa-edit-h1 = Edit WhatsApp Account
admin-wa-edit-back = ← WhatsApp Accounts
admin-wa-edit-label-name = Name
admin-wa-edit-label-phone = Phone Number
admin-wa-edit-label-phone-id = Phone Number ID
admin-wa-edit-placeholder-phone = +1234567890
admin-wa-edit-placeholder-phone-id = Meta phone number ID
admin-wa-edit-h3-auto = Auto-Reply
admin-wa-edit-rules-prefix = This is the default reply when no rule matches. Manage the full rules list at
admin-wa-edit-rules-link = Reply rules
admin-wa-edit-enabled = Enabled
admin-wa-edit-mode = Default reply mode
admin-wa-edit-mode-static = Static
admin-wa-edit-mode-ai = AI
admin-wa-edit-prompt = Default reply text / prompt
admin-wa-edit-wait-prefix = Wait before replying
admin-wa-edit-wait-help = 0 = reply immediately. Higher values let customers send a burst of messages and get one combined reply.

# Admin: Instagram list + edit.
admin-ig-list-title = Instagram Accounts - Concierge
admin-ig-list-h1 = Instagram Accounts
admin-ig-list-add = + Connect Account
admin-ig-list-th-username = Username
admin-ig-list-th-status = Status
admin-ig-list-th-auto = Auto-Reply
admin-ig-list-empty = No Instagram accounts connected.
admin-ig-list-delete-confirm = Remove this Instagram account?
admin-ig-edit-title = Edit Instagram Account - Concierge
admin-ig-edit-h1 = Edit Instagram Account
admin-ig-edit-back = ← Instagram Accounts
admin-ig-edit-account-enabled = Account Enabled
admin-ig-edit-h3-auto = Auto-Reply
admin-ig-edit-rules-prefix = This is the default reply when no rule matches. Manage the full rules list at
admin-ig-edit-rules-link = Reply rules
admin-ig-edit-enabled = Enabled
admin-ig-edit-mode = Default reply mode
admin-ig-edit-mode-static = Static
admin-ig-edit-mode-ai = AI
admin-ig-edit-prompt = Default reply text / prompt
admin-ig-edit-wait-prefix = Wait before replying
admin-ig-edit-wait-help = 0 = reply immediately. Higher values let customers send a burst of messages and get one combined reply.

# Admin: Lead forms list + edit.
admin-lf-list-title = Lead Forms - Concierge
admin-lf-list-h1 = Lead Forms
admin-lf-list-add = + New Form
admin-lf-list-back = ← Back to Dashboard
admin-lf-list-th-name = Name
admin-lf-list-th-slug = Slug
admin-lf-list-th-status = Status
admin-lf-list-empty = No lead forms created.
admin-lf-list-delete-confirm = Delete this lead form?
admin-lf-edit-title = Edit Lead Form - Concierge

# Legal: shared.
legal-back-home = ← Home

# Legal: Terms of Service.
terms-title = Terms of Service | Concierge
terms-meta-description = Terms of Service for Concierge, an automated messaging platform for small businesses.
terms-h1 = Terms of Service
terms-effective = Effective April 26, 2026
terms-h2-service = 1. Service
terms-p-service = Concierge ("the Service") is a messaging automation platform operated by Calculon Tech at concierge.calculon.tech. By using the Service you agree to these terms.
terms-h2-accounts = 2. Accounts
terms-p-accounts = You sign in with Google OAuth. You are responsible for the activity on your account and the phone numbers and Instagram accounts you connect.
terms-h2-acceptable = 3. Acceptable Use
terms-p-acceptable = You must not use the Service to send spam, unsolicited messages, or any content that violates applicable law. You must comply with Meta's WhatsApp Business Policy and Instagram Platform Policy.
terms-h2-data = 4. Data
terms-p-data = We store the minimum data needed to operate: your email, connected account metadata, and message logs. See our <a href="/privacy">Privacy Policy</a> for details. You can delete all your data at any time from Settings.
terms-h2-warranty = 5. No Warranty
terms-p-warranty = The Service is provided "as is" without warranty of any kind. We do not guarantee uptime, message delivery, or API availability.
terms-h2-ai = 6. AI-generated replies
terms-p-ai-1 = The Service uses third-party large language models to draft replies on your behalf. AI output may be incorrect, incomplete, misleading, or inappropriate for your context. You are solely responsible for the content sent from your connected accounts, including AI-drafted messages. You are responsible for reviewing your persona prompt and reply rules to ensure outputs comply with applicable law and platform policies (Meta WhatsApp Business Policy, Instagram Platform Policy, Discord Terms of Service).
terms-p-ai-2 = <strong>Calculon Tech disclaims all liability for AI-generated content sent via the Service</strong>, including without limitation factual errors, regulatory or platform-policy violations, defamatory content, missed appointments, mispriced quotes, and any commercial loss arising from AI replies. The persona safety check is a best-effort automated screen and does not constitute review or approval of any specific message.
terms-h2-liability = 7. Limitation of Liability
terms-p-liability = To the maximum extent permitted by law, Calculon Tech is not liable for any indirect, incidental, consequential, or special damages arising from your use of the Service, including damages arising from AI-generated replies and any business consequence thereof.
terms-h2-changes = 8. Changes
terms-p-changes = We may update these terms. Continued use after changes constitutes acceptance.
terms-h2-contact = 9. Contact
terms-p-contact = Questions? Open an issue at <a href="https://github.com/ananthb/concierge">github.com/ananthb/concierge</a>.

# Legal: Privacy Policy.
privacy-title = Privacy Policy | Concierge
privacy-meta-description = Privacy Policy for Concierge. We store the minimum data needed to operate and you can delete everything at any time.
privacy-h1 = Privacy Policy
privacy-effective = Effective April 29, 2026
privacy-h2-collect = What we collect
privacy-li-account = <strong>Account info:</strong> your Google account email address and display name, obtained when you sign in via Google OAuth using the <code>openid</code>, <code>email</code>, and <code>profile</code> scopes. We do not request access to Gmail, Drive, Calendar, Contacts, or any other Google Workspace data.
privacy-li-connected = <strong>Connected accounts:</strong> WhatsApp phone number IDs, Instagram page IDs, and encrypted access tokens
privacy-li-logs = <strong>Message logs:</strong> inbound/outbound WhatsApp and Instagram messages processed by auto-reply
privacy-li-leads = <strong>Lead form submissions:</strong> phone numbers submitted through your lead capture forms
privacy-li-persona = <strong>Persona prompts and reply rules:</strong> the AI persona text you write and the rule descriptions you configure
privacy-h2-use = How we use it
privacy-p-use = Solely to operate the Service: authenticating your sign-in, routing messages, generating auto-replies, and displaying your admin dashboard. Your Google account email and name are used only to identify your account and to display your name in the admin UI.
privacy-h2-storage = How we store and protect your data
privacy-p-storage = Data is stored on Cloudflare's infrastructure (D1 database and KV store). All transfers between your browser, the Service, and third-party APIs use TLS encryption. Sensitive tokens (Meta access tokens, session cookies) are encrypted at rest with AES-256-GCM. Database access is restricted to the Service's backend code and a small number of authorised maintainers; no human routinely reviews customer data.
privacy-h2-ai = AI processing
privacy-p-ai = Inbound message text and your persona prompt are sent to Cloudflare Workers AI to draft replies and to classify message intent and persona safety. Cloudflare's AI processing terms apply. Persona prompts are also classified by an automated safety scanner; the scanner's category labels are logged for abuse review and not shared. AI-generated replies may be incorrect or inappropriate. See our <a href="/terms">Terms of Service</a> for the liability disclaimer.
privacy-h2-third = How we share your data
privacy-p-third = We interact with Meta's WhatsApp and Instagram APIs and Cloudflare Workers AI on your behalf to deliver the Service. We do <strong>not</strong> sell your data, transfer it to data brokers or information resellers, or share it with any other third party, except (a) with your explicit consent, (b) as required by law or valid legal process, or (c) as part of a merger, acquisition, or sale of assets, in which case we will notify you before your data is transferred and becomes subject to a different privacy policy.
privacy-h2-google-limited = Limited use of Google user data
privacy-p-google-limited-prefix = Concierge's use of information received from Google APIs adheres to the <a href="https://developers.google.com/terms/api-services-user-data-policy">Google API Services User Data Policy</a>, including the Limited Use requirements. Specifically:
privacy-li-google-limited-1 = We use Google account data (email and name) only to provide and improve user-facing features of Concierge that are visible and prominent in the user interface.
privacy-li-google-limited-2 = We do not use Google user data to serve advertisements, including retargeting, personalised, or interest-based advertising.
privacy-li-google-limited-3 = We do not transfer Google user data to third parties except as necessary to provide or improve the Service, to comply with applicable law, or as part of a merger, acquisition, or sale of assets with notice to users.
privacy-li-google-limited-4 = We do not use Google user data, or data derived from it, to develop, improve, or train generalised or non-personalised AI or machine-learning models.
privacy-li-google-limited-5 = We do not use Google user data for credit-worthiness, lending, insurance underwriting, or any similar evaluation.
privacy-li-google-limited-6 = No human at Concierge reads your Google user data unless we have your affirmative consent for specific messages, it is necessary for security purposes (e.g. investigating abuse), to comply with applicable law, or the data is aggregated and used for internal operations in line with this policy.
privacy-h2-retention = Data retention
privacy-p-retention = Data is retained while your account is active. You can delete all your data at any time from <a href="/admin/settings">Settings</a>. When you delete your account, your Google account email, display name, and all associated tenant data are removed from our database immediately.
privacy-h2-deletion = Data deletion
privacy-p-deletion-prefix = To delete your account and all associated data:
privacy-li-deletion-1 = Go to <a href="/admin/settings">Settings</a> and click "Delete Account"
privacy-li-deletion-2 = Or remove the Concierge app from your <a href="https://www.facebook.com/settings?tab=business_tools">Facebook Business Integrations</a>
privacy-p-deletion-suffix = Deletion is immediate and irreversible.
privacy-h2-contact = Contact
privacy-p-contact = Questions? Open an issue at <a href="https://github.com/ananthb/concierge">github.com/ananthb/concierge</a>.

# Wizard step 2: Connect channels.
wizard-channels-eyebrow = Plug in
wizard-channels-headline = Where do your customers already talk to you?
wizard-channels-lead = Connect your channels. Skip anything you don't use — you can add more from the dashboard later.
wizard-channels-continue = Continue →
wizard-channels-skip = Skip →
wizard-channels-name-instagram = Instagram DMs
wizard-channels-flavor-instagram = Meta login. We'll read DMs from your business account.
wizard-channels-handle-instagram-demo = @blossom.florist
wizard-channels-name-whatsapp = WhatsApp Business
wizard-channels-flavor-whatsapp = Uses your Meta Business access token + phone number ID.
wizard-channels-handle-whatsapp-demo = +61 431 555 019
wizard-channels-name-discord = Discord
wizard-channels-flavor-discord = Install the bot to relay messages, approve AI drafts, and run slash commands.
wizard-channels-discord-handle-fallback = Connected
wizard-channels-name-email = Email
wizard-channels-email-lead-prefix = Pick a name to receive mail at
wizard-channels-email-lead-suffix = . Replies go to the sender; you and your team get a copy via Cc/Bcc.
wizard-channels-email-placeholder = your-name
wizard-channels-email-add = Add
wizard-channels-email-help = Reply-email subscription: { $pack_size } addresses per pack at { $inr } / { $usd } per month.
wizard-channels-card-active = active
wizard-channels-card-manage = Manage
wizard-channels-card-connect = Connect →
wizard-channels-card-connected = connected

# Wizard step 3: Notifications.
wizard-notifications-eyebrow = Heads up
wizard-notifications-headline = How should we notify you?
wizard-notifications-lead = Approvals are required: that's how the AI asks you before sending. Digests are optional.
wizard-notifications-card-eyebrow = AI reply approvals
wizard-notifications-card-required = *
wizard-notifications-card-lead = When the AI drafts a reply, where should we ask you to approve it? Pick at least one.
wizard-notifications-channel-discord = Discord
wizard-notifications-channel-discord-sub = real-time threads
wizard-notifications-channel-email = Email
wizard-notifications-channel-email-sub = batched digest
wizard-notifications-cadence-prefix = Send digest
wizard-notifications-discord-missing = Discord isn't installed yet. You need the bot in a server before approvals can land there.
wizard-notifications-discord-install = Install Discord
wizard-notifications-continue = Continue →

# Wizard step 4: Replies.
wizard-replies-eyebrow = Replies
wizard-replies-headline = Pick a starting style
wizard-replies-lead-prefix = Each preset comes with a tone and a small set of default reply rules. You can fine-tune everything from
wizard-replies-lead-link = Persona settings
wizard-replies-lead-suffix = after launch.
wizard-replies-goal-eyebrow = What should customers do next?
wizard-replies-goal-lead = One short verb phrase the AI can drive toward — keep it simple. Optional but strongly encouraged.
wizard-replies-goal-placeholder = e.g. book a delivery slot
wizard-replies-goal-url-label = Where should they go to do it? (optional)
wizard-replies-goal-url-placeholder = /book or https://example.com/book
wizard-replies-handoff-eyebrow = When should we call you in?
wizard-replies-handoff-lead = One condition per line. The AI will stop and notify you on these. Optional, but at least one is recommended.
wizard-replies-handoff-placeholder = refund or complaint &#10; the customer is upset &#10; order is over ₹5000
wizard-replies-wait-eyebrow = Wait before replying
wizard-replies-wait-lead = If a customer sends a few messages in a row, hold off until they pause so the AI sees the whole burst at once. Default applies to every channel; override per account in Settings.
wizard-replies-wait-instant = instant
wizard-replies-continue = Continue →

# Admin email dashboard.
admin-email-title = Email: Concierge
admin-email-h1 = Email
admin-email-lead-prefix = Each address you add at
admin-email-lead-suffix = can auto-reply to incoming mail. Replies go to the sender; you and your team get a copy via Cc/Bcc.
admin-email-th-address = Address
admin-email-th-autoreply = Auto-reply
admin-email-th-notify = Notify
admin-email-th-actions = Actions
admin-email-mode-static = Static
admin-email-mode-ai = AI
admin-email-on = on
admin-email-off = off
admin-email-recipients-suffix = recipient(s)
admin-email-row-edit = Edit
admin-email-row-delete = Delete
admin-email-delete-confirm = Delete this address? Inbound mail will be rejected.
admin-email-empty = No email addresses yet. Pick a name below: you can use it like <code>name@domain</code> from the moment you save.
admin-email-quota-warn-prefix = You've used your
admin-email-quota-warn-suffix = address slot(s). Buy a 5-pack from
admin-email-quota-warn-link = Billing
admin-email-quota-warn-tail = to add more.
admin-email-add-h2 = Add an address
admin-email-add-lead-prefix = of
admin-email-add-lead-suffix = addresses used. Pick a memorable local-part: it can use a-z, 0-9, dot, dash, underscore.
admin-email-add-placeholder = support
admin-email-add-cta = Add

# Per-address edit page.
admin-email-edit-title-suffix = : Concierge
admin-email-edit-back = ← All addresses
admin-email-edit-h1 = Address
admin-email-edit-rules-prefix = This is the default reply when no rule matches. Manage the full rules list at
admin-email-edit-rules-link = Reply rules
admin-email-edit-toggle-label = Reply automatically to inbound mail
admin-email-edit-mode-label = Default reply mode
admin-email-edit-mode-canned = Static: same canned reply every time
admin-email-edit-mode-prompt = AI: generate a reply for each message (uses 1 credit)
admin-email-edit-prompt-label = Default reply text / AI prompt
admin-email-edit-prompt-placeholder = In static mode this exact text is sent. In AI mode, this is the system prompt for the model.
admin-email-edit-wait-prefix = Wait before replying
admin-email-edit-wait-help = Lets clusters of forwarded messages collapse into one reply. 0 = reply immediately.
admin-email-edit-save = Save
admin-email-recipients-h2 = Notification recipients
admin-email-recipients-lead = Add Cc or Bcc addresses to keep your team in the loop. We send a verification email to each new address.
admin-email-recipients-th-address = Address
admin-email-recipients-th-kind = Kind
admin-email-recipients-th-status = Status
admin-email-recipients-empty = No notification recipients.
admin-email-recipients-cc = Cc
admin-email-recipients-bcc = Bcc
admin-email-recipients-status-owner = Owner
admin-email-recipients-status-verified = Verified
admin-email-recipients-status-pending = Pending
admin-email-recipients-remove = Remove
admin-email-recipients-remove-confirm = Remove this recipient?
admin-email-recipients-add-h3 = Add a recipient
admin-email-recipients-add-placeholder = team@example.com
admin-email-recipients-add-kind-cc = Cc
admin-email-recipients-add-kind-bcc = Bcc
admin-email-recipients-add-cta = Send verification email

# Email verification result page.
admin-email-verify-title = Email verification: Concierge
admin-email-verify-h1 = Email verification
admin-email-verify-back = Back to Concierge

# Admin: Persona builder.
admin-persona-title = Persona — Concierge
admin-persona-back = ← Dashboard
admin-persona-h1 = Persona
admin-persona-lead = The persona is your AI assistant's voice. Every AI-generated reply uses this prompt as its system prompt.
admin-persona-mode-eyebrow = Mode
admin-persona-mode-preset = Preset
admin-persona-mode-builder = Builder
admin-persona-mode-custom = Custom prompt
admin-persona-preset-lead = Pick one of the curated personas. The prompt and a starter set of reply rules will be applied.
admin-persona-builder-lead = Fill in the fields and we'll compose the prompt for you. Switch to Custom mode if you want to write the whole thing yourself.
admin-persona-label-biz-type = Type of business
admin-persona-label-city = City (optional)
admin-persona-label-tone = Tone
admin-persona-label-never = Never (one boundary)
admin-persona-label-catch-phrases = Catch-phrases (one per line, max 5)
admin-persona-label-off-topics = Off-topic subjects (one per line, max 10)
admin-persona-placeholder-biz-type = florist, hair salon, cafe...
admin-persona-placeholder-city = Chennai, Berlin...
admin-persona-placeholder-tone = warm and friendly, concise and professional...
admin-persona-placeholder-never = quote prices, promise dates...
admin-persona-placeholder-catch-phrases = One catch-phrase per line
admin-persona-placeholder-off-topics = politics, medical advice, refunds...
admin-persona-label-goal = Goal (what to drive customers toward)
admin-persona-placeholder-goal = book a delivery slot
admin-persona-hint-goal = Optional but strongly encouraged. Keep it short — one verb phrase the AI can hit in a couple of replies.
admin-persona-label-goal-url = Goal URL (optional)
admin-persona-placeholder-goal-url = /book or https://example.com/book
admin-persona-label-handoff = Hand off to a human if (one per line, max 5)
admin-persona-placeholder-handoff = refund or complaint &#10; the customer is upset &#10; order is over ₹5000
admin-persona-hint-handoff = Optional but recommended. The AI will stop and notify you when any of these come up.
admin-persona-custom-lead = Write the entire system prompt yourself. Up to 2000 characters.
admin-persona-custom-sr-only = System prompt
admin-persona-custom-placeholder = You are a helpful assistant for...
admin-persona-save = Save persona
admin-persona-preview-eyebrow = PROMPT PREVIEW
admin-persona-preview-refresh = Refresh preview
admin-persona-envelope-note = Concierge wraps every prompt with a fixed safety preamble and postamble. They aren't editable; you control the middle section.
admin-prompt-preamble-label = Built-in preamble (not editable)
admin-prompt-postamble-label = Built-in safety postamble (not editable)
admin-persona-safety-rechecking = Re-checking
admin-persona-safety-rechecking-detail = Prompt was edited; safety re-check in progress.
admin-persona-safety-approved = Approved
admin-persona-safety-approved-prefix = Last checked
admin-persona-safety-approved-fallback = just now
admin-persona-safety-pending = Pending
admin-persona-safety-pending-detail = Safety check in progress; AI replies are paused until it completes.
admin-persona-safety-rejected = Rejected
admin-persona-safety-rejected-fallback = This persona doesn't fit our content policies.

# Admin: Discord settings.
admin-discord-install-back = ← Back
admin-discord-install-title = Connect Discord - Concierge
admin-discord-install-h1 = Discord
admin-discord-install-card-headline = Install the Concierge bot
admin-discord-install-card-lead = Pick a server and approve the bot. We use Discord for AI approvals, digests, and email relay.
admin-discord-install-cta = Install →
admin-discord-manage-title = Discord - Concierge
admin-discord-manage-back = ← Back
admin-discord-manage-server-prefix = Server:
admin-discord-manage-uninstall = Uninstall
admin-discord-manage-uninstall-confirm = Uninstall the bot and forget these channels?
admin-discord-out-h3 = Outbound channels
admin-discord-out-lead = Pick where we should post to. Per-rule overrides still win over these defaults.
admin-discord-approval-channel = Approvals channel
admin-discord-approval-channel-help = AI drafts land here for you to approve or reject.
admin-discord-in-h3 = Inbound triggers
admin-discord-in-lead = Choose when the bot replies. DMs aren't supported with the shared bot.
admin-discord-in-mentions = Reply when @mentioned
admin-discord-in-channels = Always reply in these channels
admin-discord-in-channels-help = Hold Cmd/Ctrl to multi-select. The bot will respond to every message in each chosen channel.
admin-discord-ar-h3 = AI auto-reply
admin-discord-ar-rules-prefix = This is the default reply when no rule matches. Manage the full rules list at
admin-discord-ar-rules-link = Reply rules
admin-discord-ar-enabled = Enabled
admin-discord-ar-mode = Mode
admin-discord-ar-mode-canned = Static (canned text: doesn't consume credits)
admin-discord-ar-mode-prompt = AI (uses 1 credit per reply)
admin-discord-ar-prompt-system = System prompt
admin-discord-ar-prompt-reply = Reply text
admin-discord-ar-wait-prefix = Wait before replying:
admin-discord-ar-wait-instant = instant
admin-discord-ar-wait-help = 0 = reply immediately. Higher values let users send a burst of messages and get one combined reply.
admin-discord-empty-channels = No text channels detected. The bot might not have access yet — invite it again, or check the Discord server settings.
admin-discord-save = Save

# Admin: Rules editor.
admin-rules-title = Reply rules - Concierge
admin-rules-edit-title = Edit rule - Concierge
admin-rules-list-h1 = Reply rules
admin-rules-list-lead = Inbound messages are checked against these rules in order. The first match fires; if nothing matches, the default reply runs.
admin-rules-list-routing-h2 = Routing rules
admin-rules-list-empty = No rules yet. Add one below or rely on the default reply.
admin-rules-list-add = + Add rule
admin-rules-list-default-h2 = Default reply
admin-rules-list-back-prefix = ←
admin-rules-row-edit = Edit
admin-rules-row-delete = Delete
admin-rules-row-delete-confirm = Delete this rule?
admin-rules-row-move-up = Move rule up
admin-rules-row-move-down = Move rule down
admin-rules-chip-default = default
admin-rules-chip-keywords = keywords
admin-rules-chip-prompt = prompt
admin-rules-chip-canned = canned
admin-rules-chip-ai = AI
admin-rules-chip-auto = auto
admin-rules-chip-always-asks = always asks
admin-rules-chip-no-gate = unsafe: no gate
admin-rules-default-edit = Edit default
admin-rules-form-title-default = Edit default reply
admin-rules-form-title-edit-fallback = Edit rule
admin-rules-form-title-add = Add a rule
admin-rules-form-back = ← All rules
admin-rules-form-label = Label
admin-rules-form-label-placeholder = Pricing questions
admin-rules-form-match-by = Match by
admin-rules-form-match-keyword = Keywords
admin-rules-form-match-prompt = Prompt (AI intent)
admin-rules-form-keywords = Keywords (comma- or newline-separated)
admin-rules-form-keywords-placeholder = hours, open, closed
admin-rules-form-keywords-help = Case-insensitive. Matches if the inbound message contains any of these.
admin-rules-form-description = Description
admin-rules-form-description-placeholder = asks about hours
admin-rules-form-description-help = Describe the kind of message that should match. We embed this and compare against incoming messages.
admin-rules-form-threshold-prefix = Threshold:
admin-rules-form-threshold-help = Higher = stricter match. Default 0.72.
admin-rules-form-respond-with = Respond with
admin-rules-form-response-canned = Canned text (no AI, no credit charge)
admin-rules-form-response-prompt = AI prompt (1 credit per reply)
admin-rules-form-response-text-sr = Reply text
admin-rules-form-response-placeholder = Hi! Here's what we recommend...
admin-rules-form-response-help = This text is appended to your persona prompt and sent to the LLM.
admin-rules-form-envelope-note = Concierge wraps the combined prompt in a fixed safety preamble + postamble before it reaches the model. View the active envelope on the persona page.
admin-rules-form-cancel = Cancel
admin-rules-form-save = Save
admin-rules-approval-eyebrow = When should this AI reply send?
admin-rules-approval-auto = Auto: send unless the safety check pauses it
admin-rules-approval-auto-detail = Default. We send the draft straight away unless our heuristic spots a risk (money, commitments, off-persona) — then it goes to your approval queue.
admin-rules-approval-always = Ask me first for every reply
admin-rules-approval-always-detail = Every AI reply waits for your approval before it sends.
admin-rules-approval-no-gate = Send without any safety check
admin-rules-approval-no-gate-detail = Reply sends without our heuristic gate. Use only on rules with no money/commitment risk.
admin-rules-modal-h2 = Turn off the safety check for this rule
admin-rules-modal-lead = The risk gate normally pauses an AI reply for your review when it:
admin-rules-modal-li-1 = Mentions money, prices, refunds, or discounts
admin-rules-modal-li-2 = Makes a commitment ("guarantee", "by Friday", "confirmed")
admin-rules-modal-li-3 = Looks unusual in length
admin-rules-modal-li-4 = Drifts onto persona off-topics
admin-rules-modal-disclaimer = <strong>By turning this off you accept</strong> that AI-generated replies under this rule will be sent without our heuristic safety check. This is in addition to the disclaimers in our terms of service. Calculon Tech disclaims all liability for any reply sent under this rule, including without limitation factual errors, regulatory or platform-policy violations, defamatory content, missed appointments, mispriced quotes, and any commercial loss.
admin-rules-modal-ack-1 = I understand the safety check is off for this rule.
admin-rules-modal-ack-2 = I accept the terms above.
admin-rules-modal-cancel = Cancel
admin-rules-modal-confirm = Turn off safety check and save

# Admin: Lead form edit.
admin-lf-edit-back = ← Back to Lead Forms
admin-lf-edit-h1 = Edit Lead Form
admin-lf-edit-name = Name
admin-lf-edit-enabled = Enabled
admin-lf-edit-wa-account = WhatsApp Account
admin-lf-edit-reply-mode = Reply Mode
admin-lf-edit-reply-mode-static = Static
admin-lf-edit-reply-mode-ai = AI
admin-lf-edit-reply-prompt = Reply Prompt
admin-lf-edit-style-h3 = Style
admin-lf-edit-style-primary = Primary Color
admin-lf-edit-style-text = Text Color
admin-lf-edit-style-bg = Background Color
admin-lf-edit-style-radius = Border Radius
admin-lf-edit-style-button = Button Text
admin-lf-edit-style-placeholder = Placeholder Text
admin-lf-edit-style-success = Success Message
admin-lf-edit-style-css = Custom CSS
admin-lf-edit-origins-h3 = Allowed Origins
admin-lf-edit-origins-placeholder = https://example.com (one per line)
admin-lf-edit-origins-help = One origin per line. Leave empty to allow all.
admin-lf-edit-save = Save
admin-lf-edit-embed-h3 = Embed Code
admin-lf-edit-embed-lead = Copy and paste this into your website:
admin-lf-edit-embed-copy = Copy
