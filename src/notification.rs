pub fn send(body: impl Into<String>) {
    let body = body.into();
    if let Err(err) = notify_rust::Notification::new()
        .appname("ChalKak")
        .summary("ChalKak")
        .body(&body)
        .show()
    {
        tracing::warn!("system notification failed: {err}");
    }
}
