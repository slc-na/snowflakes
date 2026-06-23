use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

use crate::config::env::load_guacamole_url;

#[derive(Deserialize)]
struct GuacamoleTokenResponse {
    #[serde(rename = "authToken")]
    auth_token: String,
}

#[tauri::command]
pub async fn guacamole_login(
    client: tauri::State<'_, Client>,
    username: String,
    password: String,
) -> Result<String, String> {
    let guacamole_url = load_guacamole_url();
    let url = format!("{}/guacamole/api/tokens", guacamole_url.trim_end_matches('/'));

    let username = username
        .split('@')
        .next()
        .unwrap_or(&username)
        .to_string();

    let mut params = HashMap::new();
    params.insert("username", username);
    params.insert("password", password);

    let response = client
        .post(&url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        eprintln!("Guacamole login failed with status {}: {}", status, text);
        return Err(format!("Guacamole login failed with status {}: {}", status, text));
    }

    let body = response
        .json::<GuacamoleTokenResponse>()
        .await
        .map_err(|e| format!("Failed to parse Guacamole response: {}", e))?;

    Ok(body.auth_token)
}
