pub fn add_to_homepage(label: &str, icon: Option<&str>, path: Option<&str>, widget: Option<&str>) {
    crate::Request::to(("our", "homepage", "homepage", "sys"))
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
