use crate::Request;

/// Add a new icon and/or widget to the Kinode homepage. Note that the process calling this
/// function must have the `homepage:homepage:sys` messaging capability.
///
/// An icon must be a base64 encoded SVG.
///
/// A path will be automatically placed underneath the namespace of the process. For example,
/// if the process is named `my:process:pkg`, and the path given is `/mypath`, the full path
/// will be `my:process:pkg/mypath`.
///
/// A widget should be HTML: it will be displayed in an iframe.
pub fn add_to_homepage(label: &str, icon: Option<&str>, path: Option<&str>, widget: Option<&str>) {
    Request::to(("our", "homepage", "homepage", "sys"))
        .body(
            serde_json::json!({
                "Add": {
                    "label": label,
                    "icon": icon,
                    "path": path,
                    "widget": widget
                }
            })
            .to_string(),
        )
        .send()
        .unwrap();
}
