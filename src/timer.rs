use crate::*;
use anyhow::Result;

/// Set a timer using the runtime that will return a Response after the specified duration.
/// The duration should be a number of seconds.
pub fn set_timer<T>(duration: u64, context: Option<T>) -> Result<()>
where
    T: TryInto<Vec<u8>, Error = anyhow::Error>,
{
    match context {
        None => {
            Request::new()
                .target(Address::new("our", ProcessId::new("timer", "sys", "uqbar")))
                .ipc(duration.to_le_bytes())
                .expects_response(duration + 1)
                .send()?;
            Ok(())
        }
        Some(context) => {
            Request::new()
                .target(Address::new("our", ProcessId::new("timer", "sys", "uqbar")))
                .ipc(duration.to_le_bytes())
                .expects_response(duration + 1)
                .try_context(context)?
                .send()?;
            Ok(())
        }
    }
}

/// Set a timer using the runtime that will return a Response after the specified duration,
/// then wait for that timer to resolve. The duration should be a number of seconds.
pub fn set_and_await_timer(
    duration: u64,
) -> anyhow::Result<Result<(Address, Message), SendError>> {
    Request::new()
        .target(Address::new("our", ProcessId::new("timer", "sys", "uqbar")))
        .ipc(duration.to_le_bytes())
        .send_and_await_response(duration + 1)
}
