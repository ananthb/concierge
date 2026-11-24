// Boots the Elm app, and bridges its ports (Razorpay checkout + Meta Embedded Signup).
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
  if (document.body) document.body.replaceChildren();

  var reducedMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  var savedTheme = null;
  try {
    savedTheme = localStorage.getItem('replybots-theme');
  } catch (e) {}
  if (savedTheme === 'dark' || savedTheme === 'light') {
    document.documentElement.setAttribute('data-theme', savedTheme);
  }
  var isDark =
    savedTheme === 'dark'
      ? true
      : savedTheme === 'light'
        ? false
        : !!(window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);

  var app = window.Elm.Main.init({
    flags: { width: window.innerWidth, reducedMotion: reducedMotion, isDark: isDark },
  });

  // A signal that the app is live, for tests to wait on.
  window.__replybotsBooted = true;

  if (!app.ports) return;

  if (app.ports.setTheme) {
    app.ports.setTheme.subscribe(function (theme) {
      if (theme === 'dark' || theme === 'light') {
        document.documentElement.setAttribute('data-theme', theme);
        try {
          localStorage.setItem('replybots-theme', theme);
        } catch (e) {}
      } else {
        document.documentElement.removeAttribute('data-theme');
        try {
          localStorage.removeItem('replybots-theme');
        } catch (e) {}
      }
    });
  }

  // Idempotent external script loader for on-demand SDKs (Razorpay, Facebook).
  var loadingScripts = {};
  function loadScript(src, onLoad, onError) {
    if (loadingScripts[src]) {
      loadingScripts[src].push({ onLoad: onLoad, onError: onError });
      return;
    }
    loadingScripts[src] = [{ onLoad: onLoad, onError: onError }];
    var s = document.createElement('script');
    s.src = src;
    s.async = true;
    s.onload = function () {
      var cbs = loadingScripts[src] || [];
      delete loadingScripts[src];
      cbs.forEach(function (cb) { cb.onLoad(); });
    };
    s.onerror = function () {
      var cbs = loadingScripts[src] || [];
      delete loadingScripts[src];
      cbs.forEach(function (cb) { cb.onError(); });
    };
    document.head.appendChild(s);
  }

  // Razorpay hosted checkout modal.
  if (app.ports.openCheckout && app.ports.paymentOutcome) {
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
            name: 'replybots',
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

      loadScript(
        'https://checkout.razorpay.com/v1/checkout.js',
        function () {
          open(window.Razorpay);
        },
        function () {
          done({ error: 'Could not reach the payment provider.' });
        }
      );
    });
  }

  // Meta WhatsApp Embedded Signup JS SDK flow.
  if (app.ports.openMetaSignup && app.ports.metaSignupOutcome) {
    var activeMessageListener = null;

    app.ports.openMetaSignup.subscribe(function (opts) {
      var wabaId = '';
      var phoneNumberId = '';
      var settled = false;

      if (activeMessageListener) {
        window.removeEventListener('message', activeMessageListener);
        activeMessageListener = null;
      }

      function cleanup() {
        if (activeMessageListener) {
          window.removeEventListener('message', activeMessageListener);
          activeMessageListener = null;
        }
      }

      function finish(payload) {
        if (settled) return;
        settled = true;
        cleanup();
        app.ports.metaSignupOutcome.send(payload);
      }

      function onMessage(event) {
        if (
          event.origin !== 'https://www.facebook.com' &&
          event.origin !== 'https://web.facebook.com'
        ) {
          return;
        }
        var data = event.data;
        if (typeof data === 'string') {
          try {
            data = JSON.parse(data);
          } catch (e) {
            return;
          }
        }
        if (!data || data.type !== 'WA_EMBEDDED_SIGNUP') return;
        if (data.event === 'FINISH' && data.data) {
          wabaId = data.data.waba_id || '';
          phoneNumberId = data.data.phone_number_id || '';
        }
      }

      function launch() {
        if (!window.FB) {
          finish({ status: 'error', message: 'Meta signup SDK failed to initialise.' });
          return;
        }

        try {
          window.FB.init({
            appId: opts.appId,
            autoLogAppEvents: true,
            xfbml: true,
            version: 'v21.0',
          });
        } catch (e) {
          finish({ status: 'error', message: 'Could not initialise Meta signup.' });
          return;
        }

        activeMessageListener = onMessage;
        window.addEventListener('message', activeMessageListener);

        try {
          window.FB.login(
            function (response) {
              var code =
                response &&
                response.authResponse &&
                response.authResponse.code;

              if (!code) {
                finish({ status: 'cancelled' });
                return;
              }

              fetch('/whatsapp/signup/callback', {
                method: 'POST',
                credentials: 'same-origin',
                headers: {
                  'Content-Type': 'application/json',
                  'X-Replybots-Request': '1',
                  'Accept': 'application/json',
                },
                body: JSON.stringify({
                  code: code,
                  state: opts.state,
                  phone_number_id: phoneNumberId,
                  waba_id: wabaId,
                }),
              })
                .then(function (resp) {
                  if (resp.ok) {
                    finish({ status: 'ok' });
                    return null;
                  }
                  return resp
                    .json()
                    .catch(function () { return null; })
                    .then(function (body) {
                      var msg =
                        (body && body.error && body.error.message) ||
                        'Could not connect your WhatsApp Business number.';
                      finish({ status: 'error', message: msg });
                    });
                })
                .catch(function () {
                  finish({
                    status: 'error',
                    message: 'Network error while finishing WhatsApp signup.',
                  });
                });
            },
            {
              config_id: opts.configId,
              response_type: 'code',
              override_default_response_type: true,
              extras: {
                setup: {},
                featureType: '',
                sessionInfoVersion: '3',
              },
            }
          );
        } catch (e) {
          finish({ status: 'error', message: 'Could not open Meta signup popup.' });
        }
      }

      if (window.FB) {
        launch();
        return;
      }

      loadScript(
        'https://connect.facebook.net/en_US/sdk.js',
        launch,
        function () {
          finish({
            status: 'error',
            message: 'Could not load Meta signup. Please allow connect.facebook.net and try again.',
          });
        }
      );
    });
  }
})();
