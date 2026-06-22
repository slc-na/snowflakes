use reqwest::Client;
use serde_json::Value;
use tauri::{Manager, Window};

use crate::config::env::{load_backend_url, load_bastion_ip};
use crate::http::request::get_request;
use crate::http::request::put_request;

// GET method 
#[tauri::command]
pub async fn get_server(window: Window) -> Result<Value, String> {
    let client = window.state::<Client>();
    println!("GET request received at /server");

    let backend_url = load_backend_url();
    println!("Backend URL: {}", backend_url);

    let full_url = format!("{}api/v1/servers", backend_url);
    println!("Full URL: {}", full_url);

    let result = get_request(client.inner(), &full_url).await;

    match result {
        Ok(response) => {
            println!("Received response: {}", response);

            let parsed_json: Value = match serde_json::from_str(&response) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to parse JSON string: {}", e);
                    return Err("Failed to parse JSON".into());
                }
            };

            Ok(parsed_json)
        },
        Err(e) => {
            eprintln!("Error during GET request: {}", e);
            Err(e) 
        }
    }
}
#[tauri::command]
pub async fn update_server(
    window: Window,
    params: Value,
    server_id: String,
) -> Result<Value, String> {
    let client = window.state::<Client>();
    println!("PUT request received at /servers/{} with params: {}", server_id, params);

    let backend_url = load_backend_url();
    let full_url = format!("{}api/v1/servers/{}", backend_url, server_id);

    let result: Result<Value, String> = put_request(client.inner(), &full_url, &params).await;

    match result {
        Ok(parsed_json) => {
            println!("Received response: {}", parsed_json);
            Ok(parsed_json)
        }
        Err(e) => {
            eprintln!("Error during PUT request: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_bastion_ip() -> Result<String, String> {
    let bastion_ip = load_bastion_ip();
    println!("Loaded Bastion IP: {}", bastion_ip);
    Ok(bastion_ip)
}