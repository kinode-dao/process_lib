use crate::*;
use anyhow::Result;

/// Set a timer using the runtime that will return a Response after the specified duration.
/// The duration should be a number of milliseconds.
pub fn set_timer(duration: u64, context: Option<Context>) {
    match context {
        None => {
            Request::new()
                .target(Address::new("our", ProcessId::new(Some("timer"), "sys", "uqbar")))
                .ipc(duration.to_le_bytes())
                .expects_response((duration / 1000) + 1)
                // safe to unwrap this call when we know we've set both target and ipc
                .send()
                .unwrap();
        }
        Some(context) => {
            Request::new()
                .target(Address::new("our", ProcessId::new(Some("timer"), "sys", "uqbar")))
                .ipc(duration.to_le_bytes())
                .expects_response((duration / 1000) + 1)
                .context(context)
                // safe to unwrap this call when we know we've set both target and ipc
                .send()
                .unwrap();
        }
    }
}

/// Set a timer using the runtime that will return a Response after the specified duration,
/// then wait for that timer to resolve. The duration should be a number of milliseconds.
pub fn set_and_await_timer(duration: u64) -> Result<Message, SendError> {
    Request::new()
        .target(Address::new("our", ProcessId::new(Some("timer"), "sys", "uqbar")))
        .ipc(duration.to_le_bytes())
        // safe to unwrap this call when we know we've set both target and ipc
        .send_and_await_response((duration / 1000) + 1).unwrap()
}
