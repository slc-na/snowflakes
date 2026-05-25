use crate::config::env::{
    load_base_uri,
    load_client_id,
    load_redirect_uri,
    load_secret_key
};
use reqwest::Client;
use tauri::State;
use std::collections::HashMap;

pub async fn send_request_access_token(client: State<'_, Client>, code: &str) -> Result<String, String> {
    println!(
        "Sending request to exchange code for access token with code: {}",
        code
    );
    let client_id = load_client_id();
    let redirect_uri = load_redirect_uri();
    let secret_key = load_secret_key();
    let base_uri = load_base_uri();
    

    let mut params = HashMap::new();
    params.insert("code", code);
    params.insert("client_id", &client_id);
    params.insert("client_secret", &secret_key);
    params.insert("redirect_uri", &redirect_uri);
    params.insert("grant_type", "authorization_code");

    let response = client
        .post(&format!("{}oauth/token", base_uri))
        .form(&params)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            Ok(text)
        },
        Err(e) => {
            eprintln!("Failed to send token request: {}", e);
            return Err(format!("Failed to send token request: {}", e));
        }
    }
}


pub async fn get_user_detail(access_token: &str) -> Result<serde_json::Value, String> {
    let base_uri = load_base_uri();

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}api/user", base_uri))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let user_info = response.json::<serde_json::Value>().await.map_err(|e| e.to_string())?;
        Ok(user_info)
    } else {
        Err(format!(
            "Failed to fetch user details: HTTP {}",
            response.status()
        ))
    }
}


pub async fn send_request_with_refresh_token(client: State<'_, Client>, refresh_token: &str) -> Result<String, String> {
    println!(
        "Sending request to exchange refresh token for access token with refresh token: {}",
        refresh_token
    );
    let client_id = load_client_id();
    let redirect_uri = load_redirect_uri();
    let secret_key = load_secret_key();
    let base_uri = load_base_uri();
    

    let mut params = HashMap::new();
    params.insert("refresh_token", refresh_token);
    params.insert("client_id", &client_id);
    params.insert("client_secret", &secret_key);
    params.insert("redirect_uri", &redirect_uri);
    params.insert("grant_type", "refresh_token");

    let response = client
        .post(&format!("{}oauth/token", base_uri))
        .form(&params)
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            let text = resp.text().await.map_err(|e| e.to_string())?;
            Ok(text)
        },
        Err(e) => {
            eprintln!("Failed to send token request: {}", e);
            return Err(format!("Failed to send token request: {}", e));
        }
    }
    }