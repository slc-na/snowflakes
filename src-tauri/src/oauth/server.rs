use super::auth::handle_oauth_callback;
use dotenvy_macro::dotenv;
use tauri::Window;
use tauri_plugin_oauth::start_with_config;
use tauri_plugin_oauth::OauthConfig;

#[tauri::command]
pub async fn start_server(window: Window) {
    let port_server = dotenv!("PORT_TEMP_SERVER").to_string();
    let config = OauthConfig {
        ports: Some(vec![port_server.parse().unwrap_or(8080)]),
        response: Some("Login successful! You can close this tab.".into()),
    };

    let result = start_with_config(config, move |raw_url| {
        let window_for_async = window.clone();

        tauri::async_runtime::spawn(async move {
            handle_oauth_callback(window_for_async, raw_url).await;
        });
    });

    match result {
        Ok(_) => println!("Temp server started successfully."),
        Err(e) => eprintln!("Failed to start temp server: {}", e),
    }
}
