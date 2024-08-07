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
                let body_string = std::str::from_utf8_lossy(&body);
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

#[macro_export]
/// A macro for writing a process that serves a widget and completes.
/// This process should be identified in your package `manifest.json` with `on_exit` set to `None`.
///
/// Make sure the process has requested capability to message `homepage:homepage:sys`!
///
/// Example:
/// ```no_run
/// wit_bindgen::generate!({
///     path: "target/wit",
///     world: "process-v0",
/// });
///
/// kinode_process_lib::widget!("My widget", create_widget);
///
/// fn create_widget() -> String {
///     return r#"<html>
///         <head>
///             <meta name="viewport" content="width=device-width, initial-scale=1">
///             <link rel="stylesheet" href="/kinode.css">
///         </head>
///         <body>
///             <h1>Hello World!</h1>
///         </body>
///     </html>"#.to_string();
/// }
/// ```
macro_rules! widget {
    ($widget_label:expr, $create_widget_func:ident) => {
        struct Component;
        impl Guest for Component {
            fn init(_our: String) {
                use kinode_process_lib::Request;
                Request::to(("our", "homepage", "homepage", "sys"))
                    .body(
                        serde_json::json!({
                            "Add": {
                                "label": $widget_label,
                                "widget": $create_widget_func(),
                            }
                        })
                        .to_string(),
                    )
                    .send()
                    .unwrap();
            }
        }
        export!(Component);
    };
}
