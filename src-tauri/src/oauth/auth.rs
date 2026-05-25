use super::token::get_token;
use super::token::save_access_token;
use super::token::save_refresh_token;
use crate::store::user::save_user_info;
use reqwest::Client;
use tauri::Manager;
use tauri::{Emitter, Window};
use url::Url;
use crate::config::env::{
    load_base_uri,
    load_client_id,
    load_redirect_uri,
    load_response_type,
    load_scope,
    load_state
};
use crate::http::account::get_user_detail;
use crate::oauth::token::get_refresh_token;
use crate::http::account::send_request_access_token;
use crate::http::account::send_request_with_refresh_token;

#[tauri::command]
pub fn open_oauth_login() -> Result<String, String> {
    let client_id = load_client_id();
    let redirect_uri = load_redirect_uri();
    let scope = load_scope();
    let response_type = load_response_type();
    let state = load_state();
    let base_uri = load_base_uri();
    let mut url = Url::parse(&format!("{}oauth/authorize", base_uri)).map_err(|e| e.to_string())?;

    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", &scope)
        .append_pair("response_type", &response_type)
        .append_pair("state", &state);

    Ok(url.as_str().into())
}

#[tauri::command]
pub async fn oauth_is_authenticated(window: Window) -> bool {
    let client = window.state::<Client>();

    let Ok(access_token) = get_token().await else {
        println!("No access token found.");
        return false;
    };

    println!("Access token found: {}", access_token);

    if get_user_detail(&access_token).await.is_ok() {
        return true; // Success! Token is valid.
    }

    eprintln!("Failed to retrieve user info. Attempting to use refresh token...");

    let Ok(refresh_token) = get_refresh_token().await else {
        eprintln!("No refresh token found to perform a refresh.");
        return false;
    };

    println!("Refresh token found: {}", refresh_token);

    let Ok(refresh_text) = send_request_with_refresh_token(client, &refresh_token).await else {
        eprintln!("Failed to get a response from the refresh token request.");
        return false;
    };

    println!("Access token response from refresh token: {}", refresh_text);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&refresh_text) else {
        eprintln!("Failed to parse refresh token response as JSON.");
        return false;
    };

    let new_access_token = json.get("access_token").and_then(|v| v.as_str());
    let new_refresh_token = json.get("refresh_token").and_then(|v| v.as_str());

    let Some(valid_new_access) = new_access_token else {
        eprintln!("Access token not found in the refresh response.");
        return false; // Cannot proceed without a new access token
    };

    if let Err(e) = save_access_token(valid_new_access) {
        eprintln!("Failed to save new access token: {}", e);
    } else {
        println!("New access token saved successfully.");
    }

    if let Some(valid_new_refresh) = new_refresh_token {
        if let Err(e) = save_refresh_token(valid_new_refresh) {
            eprintln!("Failed to save new refresh token: {}", e);
        } else {
            println!("New refresh token saved successfully.");
        }
    } else {
        eprintln!("Warning: No new refresh token provided in the response.");
    }

    match get_user_detail(valid_new_access).await {
        Ok(_) => {
            println!("Successfully verified new access token.");
            true
        }
        Err(e) => {
            eprintln!("Failed to retrieve user info even with the new access token: {}", e);
            false
        }
    }
}
pub async fn handle_oauth_callback(window: Window, raw_url: String) {
    let app = window.app_handle().clone();

    let Ok(parsed_url) = Url::parse(&raw_url) else {
        eprintln!("Failed to parse the incoming URL.");
        return;
    };
    let Some(auth_code) = parsed_url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
    else {
        eprintln!("No auth code found in the URL.");
        return;
    };

    println!("Successfully extracted Auth Code: {}", auth_code);

    let client = window.state::<Client>();
    let Ok(text) = send_request_access_token(client.clone(), &auth_code).await else {
        eprintln!("Failed to start access and refresh token retrieval process.");
        return;
    };

    println!("Access token response: {}", text);

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        eprintln!("Failed to parse token response as JSON.");
        return;
    };

    if let Some(refresh_token) = json.get("refresh_token").and_then(|v| v.as_str()) {
        println!("Extracted Refresh Token: {}", refresh_token);
        if let Err(e) = save_refresh_token(refresh_token) {
            eprintln!("Failed to save refresh token: {}", e);
        } else {
            println!("Refresh token saved successfully.");
        }
    } else {
        eprintln!("Refresh token not found in response.");
    }

    let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) else {
        eprintln!("Access token not found in response.");
        return;
    };

    println!("Extracted Access Token: {}", access_token);
    
    if let Err(e) = save_access_token(access_token) {
        eprintln!("Failed to save access token: {}", e);
    } else {
        println!("Access token saved successfully.");
    }

    match get_user_detail(access_token).await {
        Ok(user_info) => {
            println!("User info retrieved successfully: {}", user_info);
            
            // Save user info
            if let Err(e) = save_user_info(app, user_info.clone()) {
                eprintln!("Failed to save user info: {}", e);
            }

            // Emit success event
            if let Err(e) = window.emit("oauth-success", user_info) {
                eprintln!("Failed to emit oauth-success event: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to retrieve user info: {}", e);
            
            // Emit error event
            if let Err(emit_err) = window.emit("oauth-error", format!("Failed to retrieve user info: {}", e)) {
                eprintln!("Failed to emit oauth-error event: {}", emit_err);
            }
        }
    }
}