//! Per-tenant Server-Sent Events fan-out for the approvals page.
//!
//! Each connected `/admin/approvals` browser tab opens an SSE stream that
//! routes through this Durable Object. When `approvals::enqueue` queues a
//! new draft, or any surface (Discord button, web button) resolves one,
//! the caller posts to this DO's `/broadcast` endpoint. The DO writes a
//! single SSE event to every open writer; clients re-fetch the list.
//!
//! Design notes:
//! - Singleton per tenant: id derived from `tenant_id` via `id_from_name`.
//! - In-memory only: the live writer set lives in a `RefCell<Vec<...>>` on
//!   `&self`. If the DO evicts, all SSE connections drop; the browsers'
//!   `EventSource` reconnects automatically and the new DO instance starts
//!   with an empty writer set.
//! - Event payloads are tiny pings ("data: {}\n\n"). The list HTML lives
//!   in the worker's `/admin/approvals/list` endpoint. The browser fires
//!   `hx-trigger="sse:approval-changed"` to refetch it. Keeping template
//!   logic out of the DO means the DO knows nothing about what changed,
//!   only that *something* changed for this tenant.

use std::cell::RefCell;

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::stream::StreamExt;
use worker::*;

const SSE_PING: &[u8] = b"event: approval-changed\ndata: {}\n\n";

/// Cap on simultaneous SSE writers per tenant DO. A normal session has one
/// or two open tabs; this exists so a misbehaving client can't grow the
/// in-memory list without bound.
const MAX_SUBSCRIBERS: usize = 64;

#[durable_object]
pub struct ApprovalsDO {
    // `state` and `env` aren't read directly: the DO has no persistent
    // storage and doesn't reach back into the worker. Hold them on the
    // struct anyway so the trait's `new(state, env)` contract has
    // somewhere to put them.
    #[allow(dead_code)]
    state: State,
    #[allow(dead_code)]
    env: Env,
    /// One sender per live SSE client. `RefCell` is fine because the DO
    /// runtime is single-threaded inside its V8 isolate.
    subscribers: RefCell<Vec<UnboundedSender<Vec<u8>>>>,
}

impl DurableObject for ApprovalsDO {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            subscribers: RefCell::new(Vec::new()),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let url = req.url()?;
        let path = url.path();

        match (req.method(), path) {
            (Method::Get, "/subscribe") => self.handle_subscribe(),
            (Method::Post, "/broadcast") => self.handle_broadcast(),
            _ => Response::error("Not Found", 404),
        }
    }
}

impl ApprovalsDO {
    fn handle_subscribe(&self) -> Result<Response> {
        let (tx, rx) = unbounded::<Vec<u8>>();
        {
            let mut subs = self.subscribers.borrow_mut();
            if subs.len() >= MAX_SUBSCRIBERS {
                // Evict the oldest sender. Its receiver is dropped with
                // it, so the corresponding browser sees the stream end
                // and reconnects (which goes to the back of the line).
                subs.remove(0);
            }
            subs.push(tx);
        }

        let stream = rx.map(|chunk| Ok::<Vec<u8>, Error>(chunk));
        let mut resp = Response::from_stream(stream)?;

        let headers = resp.headers_mut();
        headers.set("Content-Type", "text/event-stream")?;
        headers.set("Cache-Control", "no-store")?;
        headers.set("X-Accel-Buffering", "no")?;
        Ok(resp)
    }

    fn handle_broadcast(&self) -> Result<Response> {
        // Iterate, send to each subscriber, drop any that failed (client
        // disconnected, receiver dropped, so unbounded_send returns
        // Err). Doing this in one pass with retain_mut keeps the
        // live set tight without a second iteration.
        let mut subs = self.subscribers.borrow_mut();
        subs.retain_mut(|tx| tx.unbounded_send(SSE_PING.to_vec()).is_ok());
        Response::ok("")
    }
}
