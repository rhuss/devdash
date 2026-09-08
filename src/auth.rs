use crate::source::SourceError;

pub async fn acquire_token() -> Result<String, SourceError> {
    let mut tried = Vec::new();

    match tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
            tried.push("gh auth token (returned empty)");
        }
        Ok(output) => {
            tried.push("gh auth token (exited with error)");
            tracing::debug!("gh auth token failed with status {}", output.status);
        }
        Err(_) => {
            tried.push("gh auth token (command not found)");
        }
    }

    match std::env::var("GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => return Ok(token),
        Ok(_) => tried.push("GITHUB_TOKEN (set but empty)"),
        Err(_) => tried.push("GITHUB_TOKEN (not set)"),
    }

    Err(SourceError::NoCredential {
        tried: tried.join(", "),
    })
}
