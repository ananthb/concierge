// Boots the Elm app, and bridges its one port.
//
// This lives in a file rather than inline in the shell for a specific reason:
// the prerendered marketing pages (index.html, pricing.html, …) are served
// straight off Cloudflare's asset handler without invoking the Worker, so
// there is nothing to stamp a per-response CSP nonce into them. An external
// script needs no nonce, which lets every page — static or Worker-rendered —
// run under one policy with no inline script at all.
//
// That policy is strictly tighter than what it replaced: `script-src 'self'`
// admits nothing but our own files, where `'self' 'nonce-…'` also admitted
// whatever inline block carried the right nonce.
(function () {
  'use strict';

  if (!window.Elm || !window.Elm.Main) return;

  // Clear whatever the document already holds before handing <body> to Elm.
  //
  // The prerendered marketing pages ship with Elm's own rendered markup in the
  // body so that a crawler — which never runs this file — can read the page.
  // Elm does not hydrate, though: `Browser.application` expects to own an
  // empty body, and when it finds a foreign tree there it renders without
  // taking ownership. The page then looks perfect and is inert: link clicks
  // fall through to the browser as full navigations, so every route change is
  // a page load and the app never behaves as a SPA.
  //
  // Clearing first costs at most one frame of blank — the markup Elm renders
  // immediately afterwards is the same markup that was just removed, because
  // the snapshot was taken from this app's own output.
  if (document.body) document.body.replaceChildren();

  // No `node`. This is a `Browser.application`, which ignores that field and
  // takes over <body> wholesale — passing a mount point would imply the app
  // renders into it, and it does not.
  //
  // That matters for the prerendered pages: their body already holds rendered
  // markup and no placeholder div, so anything that required one here would
  // leave those pages static and their navigation dead.
  //
  // Flags carry what the app cannot discover for itself: the viewport width,
  // so the first render isn't a desktop layout that immediately reflows on a
  // phone. Everything else comes from GET /api/bootstrap.
  var app = window.Elm.Main.init({
    flags: { width: window.innerWidth },
  });

  // A signal that the app is live, for tests to wait on.
  //
  // It has to be a JS global rather than anything in the DOM. The prerendered
  // pages are snapshots of this app's own output, so every marker rendered
  // into the markup is present before the bundle has run — a test waiting on
  // one would pass against an inert page and miss the very failure that
  // matters most here, a static page whose navigation is dead. A global
  // cannot be captured by a DOM snapshot, so it means what it says.
  window.__conciergeBooted = true;

  if (!app.ports || !app.ports.openCheckout) return;

  // Razorpay's checkout is a hosted modal its own SDK opens, so it has to be
  // a port — there is no redirect-only flow to use instead. The SDK loads on
  // demand: most sessions never reach billing.
  //
  // Every outcome — success, failure, dismissal — sends exactly one message
  // back, so the Elm side always leaves its "working" state. A success is
  // still only a claim here; the app confirms it against
  // POST /api/billing/verify, and credits are granted by the Razorpay
  // webhook rather than by anything this file can say.
  app.ports.openCheckout.subscribe(function (order) {
    function done(result) {
      app.ports.paymentOutcome.send({
        orderId: result.orderId || order.order_id || '',
        paymentId: result.paymentId || '',
        signature: result.signature || '',
        error: result.error || '',
      });
    }

    function open(Rzp) {
      try {
        new Rzp({
          key: order.key_id,
          order_id: order.order_id,
          amount: order.amount,
          currency: order.currency,
          name: 'Concierge',
          handler: function (r) {
            done({
              orderId: r.razorpay_order_id,
              paymentId: r.razorpay_payment_id,
              signature: r.razorpay_signature,
            });
          },
          modal: {
            ondismiss: function () {
              done({});
            },
          },
        }).open();
      } catch (e) {
        done({ error: 'Could not open the payment window.' });
      }
    }

    if (window.Razorpay) {
      open(window.Razorpay);
      return;
    }

    var s = document.createElement('script');
    s.src = 'https://checkout.razorpay.com/v1/checkout.js';
    s.onload = function () {
      open(window.Razorpay);
    };
    s.onerror = function () {
      done({ error: 'Could not reach the payment provider.' });
    };
    document.head.appendChild(s);
  });
})();
