use super::token::get_token;
use super::token::save_access_token;
use super::token::save_refresh_token;
use crate::HashMap;
use dotenvy_macro::dotenv;
use reqwest::Client;
use tauri::Manager;
use tauri::State;
use tauri::{Emitter, Window};
use url::Url;

fn load_env_config() -> (String, String, String, String, String, String, String) {
    let client_id = dotenv!("CLIENT_ID").to_string();
    let redirect_uri = dotenv!("REDIRECT_URI").to_string();
    let scope = dotenv!("SCOPE").to_string();
    let response_type = dotenv!("RESPONSE_TYPE").to_string();
    let state = dotenv!("STATE").to_string();
    let secret_key = dotenv!("SECRET_KEY").to_string();
    let base_uri = dotenv!("BASE_URI").to_string();

    (
        client_id,
        redirect_uri,
        scope,
        response_type,
        state,
        secret_key,
        base_uri,
    )
}

pub async fn send_request_access(client: State<'_, Client>, code: &str) -> Result<String, String> {
    println!(
        "Sending request to exchange code for access token with code: {}",
        code
    );
    let (client_id, redirect_uri, _scope, _response_type, _state, secret_key, base_uri) =
        load_env_config();

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
        .await
        .map_err(|e| e.to_string())?;

    println!("Response: {}", response.status().as_str());

    let text = response.text().await.map_err(|e| e.to_string())?;
    Ok(text)
}

#[tauri::command]
pub fn open_oauth_login() -> Result<String, String> {
    let (client_id, redirect_uri, scope, response_type, state, _secret_key, base_uri) =
        load_env_config();

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
pub async fn oauth_is_authenticated() -> bool {
    let result = get_token().await;
    match result {
        Ok(token) => {
            println!("Access token found: {}", token);
            let user_info_result = get_user_detail(&token).await;
            match user_info_result {
                Ok(user_info) => {
                    println!("User info retrieved successfully: {}", user_info);
                    return true;
                }
                Err(e) => {
                    eprintln!("Failed to retrieve user info: {}", e);
                    return false;
                }
            }
        }
        Err(e) => {
            println!("No access token found: {}", e);
        }
    }

    false
}

pub async fn get_user_detail(access_token: &str) -> Result<serde_json::Value, String> {
    let (_, _, _, _, _, _secret_key, base_uri) = load_env_config();

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

pub async fn handle_oauth_callback(window: Window, raw_url: String) {
    if let Ok(parsed_url) = Url::parse(&raw_url) {
        let client = window.state::<Client>();
        let auth_code = parsed_url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.into_owned());

        match auth_code {
            Some(code) => {
                println!("Successfully extracted Auth Code: {}", code);

                // send post to accounts.slc.net
                let result = send_request_access(client, &code).await;

                match result {
                    Ok(text) => {
                        println!("Access token response: {}", text.clone());

                        //save the access token
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(access_token) =
                                json.get("access_token").and_then(|v| v.as_str())
                            {
                                println!("Extracted Access Token: {}", access_token);
                                if let Err(e) =
                                    save_access_token(access_token).map_err(|e| e.to_string())
                                {
                                    eprintln!("Failed to save access token: {}", e);
                                } else {
                                    let user_info_result = get_user_detail(access_token).await;
                                    match user_info_result {
                                        Ok(user_info) => {
                                            println!("User info retrieved successfully: {}", user_info);
                                            window.emit("oauth-success", user_info).unwrap_or_else(|e| {
                                                eprintln!("Failed to emit oauth-success event: {}", e);
                                            });
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to retrieve user info: {}", e);
                                            window.emit("oauth-error", format!("Failed to retrieve user info: {}", e)).unwrap_or_else(|e| {
                                                eprintln!("Failed to emit oauth-error event: {}", e);
                                            });
                                        }
                                    }
                                    println!("Access token saved successfully.");
                                }
                            } else {
                                eprintln!("Access token not found in response.");
                            }

                            if let Some(refresh_token) =
                                json.get("refresh_token").and_then(|v| v.as_str())
                            {
                                println!("Extracted Refresh Token: {}", refresh_token);
                                if let Err(e) =
                                    save_refresh_token(refresh_token).map_err(|e| e.to_string())
                                {
                                    eprintln!("Failed to save refresh token: {}", e);
                                } else {
                                    println!("Refresh token saved successfully.");
                                }
                            } else {
                                eprintln!("Refresh token not found in response.");
                            }
                        } else {
                            eprintln!("Failed to parse refresh token response as JSON.");
                        }
                    }
                    Err(e) => eprintln!(
                        "Failed to start access and refresh token retrieval process: {}",
                        e
                    ),
                }
            }
            None => {
                eprintln!("No auth code found in the URL.");
            }
        }
    } else {
        eprintln!("Failed to parse the incoming URL.");
    }
}
