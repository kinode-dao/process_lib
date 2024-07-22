#[macro_export]
/// A macro for writing a "script" process. Using this will create the initial
/// entry point for your process, including the standard `init` function which
/// is called by the system, and a set of calls that:
/// 1. Parse the `our` string into an `Address` object.
/// 2. Wait for the first message to be sent to the process.
/// 3. Convert the message body into a string.
/// 4. Call the `init` function you provide with the `Address` and the message body string.
///
/// This is best used by then using `clap` to create a `Command` and parsing the body string with it.
macro_rules! script {
    ($init_func:ident) => {
        struct Component;
        impl Guest for Component {
            fn init(our: String) {
                use kinode_process_lib::{await_message, println, Address, Message, Response};
                let our: Address = our.parse().unwrap();
                let Message::Request {
                    body,
                    expects_response,
                    ..
                } = await_message().unwrap()
                else {
                    return;
                };
                let body_string =
                    format!("{} {}", our.process(), std::str::from_utf8(&body).unwrap());
                let response_string: String = $init_func(our, body_string);
                if expects_response.is_some() {
                    Response::new()
                        .body(response_string.as_bytes())
                        .send()
                        .unwrap();
                } else {
                    println!("{response_string}");
                }
            }
        }
        export!(Component);
    };
}
