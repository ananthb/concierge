//! Durable Object classes used by the worker.
//!
//! Each DO needs to be exported via `wasm_bindgen` from `src/lib.rs` (the
//! `#[durable_object]` macro generates that wrapper).

pub mod approvals_do;
pub mod reply_buffer;

// `#[durable_object]` generates the wasm-bindgen wrappers for these
// classes; we don't `use` them directly from Rust code.
#[allow(unused_imports)]
pub use approvals_do::ApprovalsDO;
#[allow(unused_imports)]
pub use reply_buffer::ReplyBufferDO;
