use crate::*;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimerAction {
    Debug,
    SetTimer(u64),
}

/// Set a timer using the runtime that will return a Response after the specified duration.
/// The duration should be a number of milliseconds.
pub fn set_timer(duration: u64, context: Option<Context>) {
    match context {
        None => {
            Request::new()
                .target(Address::new(
                    "our",
                    ProcessId::new(Some("timer"), "distro", "sys"),
                ))
                .body(serde_json::to_vec(&TimerAction::SetTimer(duration)).unwrap())
                .expects_response((duration / 1000) + 1)
                // safe to unwrap this call when we know we've set both target and body
                .send()
                .unwrap();
        }
        Some(context) => {
            Request::new()
                .target(Address::new(
                    "our",
                    ProcessId::new(Some("timer"), "distro", "sys"),
                ))
                .body(serde_json::to_vec(&TimerAction::SetTimer(duration)).unwrap())
                .expects_response((duration / 1000) + 1)
                .context(context)
                // safe to unwrap this call when we know we've set both target and body
                .send()
                .unwrap();
        }
    }
}

/// Set a timer using the runtime that will return a Response after the specified duration,
/// then wait for that timer to resolve. The duration should be a number of milliseconds.
pub fn set_and_await_timer(duration: u64) -> Result<Message, SendError> {
    Request::new()
        .target(Address::new(
            "our",
            ProcessId::new(Some("timer"), "distro", "sys"),
        ))
        .body(serde_json::to_vec(&TimerAction::SetTimer(duration)).unwrap())
        // safe to unwrap this call when we know we've set both target and body
        .send_and_await_response((duration / 1000) + 1)
        .unwrap()
}
