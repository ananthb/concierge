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
nav-docs = Docs ↑
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
meta-default-description = Automated customer messaging for small businesses. Auto-reply across WhatsApp, Instagram DMs, and email. 100 replies free every month.

# Inline JS-driven UI strings (rendered into data-* attributes server-side
# so the script can read them without re-fetching the bundle).
js-copy-button-default = Copy
js-copy-button-copied = Copied!
js-htmx-error-toast = Something went wrong. Please try again.

# Welcome / landing page (/).
welcome-eyebrow = // automated customer engagement
welcome-headline = Hello. I'll be answering <br>every <em>DM, WhatsApp &amp; email</em> <br>so you don't have to.
welcome-lead = Concierge is an automated customer-messaging service for small businesses. Connect your channels, set a tone, and I'll auto-reply across WhatsApp, Instagram, Discord, and email. 100 AI replies free every month; static replies always free.
welcome-cta-primary = Get started →
welcome-cta-secondary = See features

# Features page (/features).
features-meta-description = Concierge auto-replies on WhatsApp, Instagram, Discord, and email. Static replies free; AI replies ₹2 / $0.02 with 100 free per month. 5-minute setup. Open source.
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
features-card-pay-body = Static auto-replies are free, forever. AI replies cost ₹2 / $0.02 each, with 100 free per month.
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
pricing-meta-description = Simple, prepaid pricing for Concierge. ₹2 / $0.02 per AI reply, no tiers. Static auto-replies free. 100 free AI replies every month. 1 free email address; ₹99 / $1 per extra address.
pricing-og-title = Concierge Pricing
pricing-headline-prefix = per AI reply. Everything else is free.
pricing-currency-inr-label = Indian rupees
pricing-currency-usd-label = US dollars
pricing-lead = 100 free AI replies every account every month. After that, top up with as many credits as you want: no tiers, no contracts. Purchased credits never expire.
pricing-credits-eyebrow = What costs a credit?
pricing-credits-li-1 = <strong>AI auto-replies</strong> on WhatsApp, Instagram, email, or Discord: <strong>1 credit each.</strong>
pricing-credits-li-2 = <strong>Static auto-replies</strong> (canned text you wrote yourself): always <strong>free</strong>.
pricing-credits-li-3 = Inbound messages, notification CCs/BCCs, Discord relay, slash commands: always <strong>free</strong>.
pricing-email-heading = Email addresses
pricing-email-body = Each address you set up at <code>name@cncg.email</code> can auto-reply to inbound mail. Replies go to the original sender; you and your team get a copy via Cc/Bcc.
pricing-email-quota-prefix = <strong>1 address free per account.</strong> Need more?
pricing-email-quota-suffix = per extra address, one-time, never expires.
pricing-email-billing-note = Static replies stay free; AI replies draw from your credit balance above.
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
wizard-launch-credits-note = Optional: 100 replies are free every month. Top up later from Billing if you need.

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
admin-login-google = Sign in with Google
admin-login-facebook-continue = Continue with Facebook
admin-login-facebook-secondary = Sign in with Facebook
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
privacy-effective = Effective April 26, 2026
privacy-h2-collect = What we collect
privacy-li-account = <strong>Account info:</strong> your Google email and name (from OAuth sign-in)
privacy-li-connected = <strong>Connected accounts:</strong> WhatsApp phone number IDs, Instagram page IDs, and encrypted access tokens
privacy-li-logs = <strong>Message logs:</strong> inbound/outbound WhatsApp and Instagram messages processed by auto-reply
privacy-li-leads = <strong>Lead form submissions:</strong> phone numbers submitted through your lead capture forms
privacy-li-persona = <strong>Persona prompts and reply rules:</strong> the AI persona text you write and the rule descriptions you configure
privacy-h2-use = How we use it
privacy-p-use = Solely to operate the Service: routing messages, generating auto-replies, and displaying your admin dashboard. We do not sell, share, or use your data for advertising.
privacy-h2-storage = Where it's stored
privacy-p-storage = Data is stored on Cloudflare's infrastructure (D1 database and KV store). Sensitive tokens are encrypted with AES-256-GCM.
privacy-h2-ai = AI processing
privacy-p-ai = Inbound message text and your persona prompt are sent to Cloudflare Workers AI to draft replies and to classify message intent and persona safety. Cloudflare's AI processing terms apply. Persona prompts are also classified by an automated safety scanner; the scanner's category labels are logged for abuse review and not shared. AI-generated replies may be incorrect or inappropriate. See our <a href="/terms">Terms of Service</a> for the liability disclaimer.
privacy-h2-third = Third parties
privacy-p-third = We interact with Meta's WhatsApp and Instagram APIs on your behalf. We use Cloudflare Workers AI for AI-powered auto-replies and intent classification. No other third parties receive your data.
privacy-h2-retention = Data retention
privacy-p-retention = Data is retained while your account is active. You can delete all your data at any time from <a href="/admin/settings">Settings</a>.
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
wizard-channels-email-help = First address is free. Need more? ₹99 / $1 per extra, one-time.
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
wizard-replies-wait-eyebrow = Wait before replying
wizard-replies-wait-lead = If a customer sends a few messages in a row, hold off until they pause so the AI sees the whole burst at once. Default applies to every channel; override per account in Settings.
wizard-replies-wait-instant = instant
wizard-replies-continue = Continue →
