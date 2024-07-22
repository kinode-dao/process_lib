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
                let our: Address = our.parse().unwrap();
                let body: Vec<u8> = await_next_message_body().unwrap();
                let body_string = format!("{} {}", our.process(), String::from_utf8(body).unwrap());
                $init_func(our, body);
            }
        }
        export!(Component);
    };
}
