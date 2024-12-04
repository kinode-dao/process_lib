use crate::{Context, Message, Request, SendError};
use serde::{Deserialize, Serialize};

/// The [`Request::body()`] field for requests to `timer:distro:sys`, a runtime module
/// that allows processes to set timers with a duration specified in milliseconds.
///
/// The timer module is public, so no particular capabilities are required to use it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimerAction {
    Debug,
    SetTimer(u64),
}

impl Into<Vec<u8>> for TimerAction {
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

/// Set a timer using the runtime that will return a [`crate::Response`] after the specified duration.
/// The duration should be a number of milliseconds.
pub fn set_timer(duration: u64, context: Option<Context>) {
    let mut request = Request::to(("our", "timer", "distro", "sys"))
        .body(TimerAction::SetTimer(duration))
        .expects_response((duration / 1000) + 1);

    if let Some(context) = context {
        request = request.context(context);
    }
    // safe to unwrap this call when we know we've set both target and body
    request.send().unwrap();
}

/// Set a timer using the runtime that will return a [`crate::Response`] after the specified duration,
/// then wait for that timer to resolve. The duration should be a number of milliseconds.
pub fn set_and_await_timer(duration: u64) -> Result<Message, SendError> {
    Request::to(("our", "timer", "distro", "sys"))
        .body(TimerAction::SetTimer(duration))
        .send_and_await_response((duration / 1000) + 1)
        // safe to unwrap this call when we know we've set both target and body
        .unwrap()
}
