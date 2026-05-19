use tauri_plugin_oauth::OauthConfig;
use tauri_plugin_oauth::start_with_config;
use url::Url;
use tauri::{Emitter, Window};
use super::token::save_access_token;
use dotenvy_macro::dotenv;

#[tauri::command]
pub async fn start_server(window: Window) {
    let port_server = dotenv!("PORT_TEMP_SERVER").to_string();
    let config = OauthConfig {
        ports: Some(vec![port_server.parse().unwrap_or(8080)]),
        response: Some("Login successful! You can close this tab.".into()),
    };

    let result = start_with_config(config, move |raw_url| {
        if let Ok(parsed_url) = Url::parse(&raw_url) {
            
            let auth_code = parsed_url.query_pairs()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.into_owned());

            match auth_code {
                Some(code) => {
                    println!("Successfully extracted Auth Code: {}", code);

                    save_access_token(&code).unwrap_or_else(|e| eprintln!("Failed to save token: {}", e));
                    
                    let _ = window.emit("oauth_success", code);
                }
                None => {
                    eprintln!("No auth code found in the URL.");
                    let _ = window.emit("oauth_error", "No code in URL");
                }
            }
        } else {
            eprintln!("Failed to parse the incoming URL.");
            let _ = window.emit("oauth_error", "Invalid URL format");
        }
    });

    match result {
        Ok(_) => println!("Temp server started successfully."),
        Err(e) => eprintln!("Failed to start temp server: {}", e),
    }
}