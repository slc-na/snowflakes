use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

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

// Iframe-based token injection is blocked by the browser's same-origin policy
// (a parent document cannot touch a cross-origin iframe's localStorage). A real
// Tauri window has no such restriction since `eval` runs inside the window's own
// page context, so we open Guacamole in its own webview window instead and inject
// the token there once it finishes loading.
#[tauri::command]
pub async fn open_guacamole_window(
    app: AppHandle,
    label: String,
    token: String,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(&label) {
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let guacamole_url = load_guacamole_url();
    let url = guacamole_url
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;

    let injected = Arc::new(AtomicBool::new(false));

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(url))
        .title("Remote Desktop")
        .inner_size(1280.0, 800.0)
        .on_page_load(move |window, payload| {
            if payload.event() == PageLoadEvent::Finished
                && !injected.swap(true, Ordering::SeqCst)
            {
                // Guacamole's localStorageService JSON.parses whatever it reads back,
                // so the stored value itself has to be JSON-encoded (i.e. wrapped in
                // an extra layer of quotes), not just the raw token string.
                let script = format!(
                    "window.localStorage.setItem('GUAC_AUTH_TOKEN', JSON.stringify({})); window.location.reload();",
                    serde_json::to_string(&token).unwrap_or_default()
                );
                let _ = window.eval(&script);
            }
        })
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn close_guacamole_window(app: AppHandle, label: String) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(&label) {
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn focus_guacamole_window(app: AppHandle, label: String) -> Result<bool, String> {
    if let Some(win) = app.get_webview_window(&label) {
        win.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}
