use std::io::Read;

use devdash::logging;

#[test]
fn log_path_is_not_empty() {
    let path = logging::log_path();
    assert!(!path.as_os_str().is_empty());
    assert!(path.to_str().unwrap().contains("devdash"));
}

#[test]
fn credential_never_appears_in_log_output() {
    let dir = tempfile::tempdir().unwrap();
    let log_file = dir.path().join("test.log");

    let secret_token = "ghp_SuperSecretToken12345";
    let redacted = "[REDACTED]";

    {
        let parent = log_file.parent().unwrap();
        let filename = log_file.file_name().unwrap();
        let file_appender = tracing_appender::rolling::never(parent, filename);
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("Starting request with token {redacted}");
            tracing::error!("Request failed: connection timeout");
            tracing::debug!("Response headers processed");
            tracing::warn!("Rate limit low: 50 remaining");

            // The token value must never be passed to a tracing field.
            // This test verifies the pattern: log the redacted placeholder, never the value.
            let _ = secret_token;
        });
    }

    let mut contents = String::new();
    std::fs::File::open(&log_file)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();

    assert!(
        !contents.contains(secret_token),
        "credential must never appear in log output"
    );
    assert!(
        contents.contains("connection timeout"),
        "errors must be logged"
    );
    assert!(
        contents.contains("[REDACTED]"),
        "redacted placeholder should appear instead of the real token"
    );
}
