use reqwest::Client;
use serde_json::Value;
use tauri::{Manager, Window};

use crate::config::env::load_backend_url;
use crate::http::request::get_request;
use crate::http::request::post_request;
use crate::http::request::put_request;

// GET method 
#[tauri::command]
pub async fn get_shell(window: Window) -> Result<Value, String> {
    let client = window.state::<Client>();
    println!("GET request received at /shell");

    let backend_url = load_backend_url();
    println!("Backend URL: {}", backend_url);

    let full_url = format!("{}api/v1/shell", backend_url);
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

// POST method 
#[tauri::command] 
pub async fn update_shell(
    window: tauri::Window, 
    params: Value,
    shell_id: String
) -> Result<Value, String> {
    
    let client = window.state::<Client>();
    println!("PUT request received at /shell/{} with params: {}", shell_id, params);

    let backend_url = load_backend_url();
    let full_url = format!("{}api/v1/shell/{}", backend_url, shell_id);
    
    let result: Result<Value, String> = put_request(client.inner(), &full_url, &params).await;

    match result {
        Ok(parsed_json) => {
            // No need for serde_json::from_str anymore!
            println!("Received response: {}", parsed_json);
            Ok(parsed_json)
        },
        Err(e) => {
            eprintln!("Error during PUT request: {}", e);
            Err(e) 
        }
    }
}

#[tauri::command] 
pub async fn insert_shell(
    window: tauri::Window, 
    params: Value
) -> Result<Value, String> {
    
    let client = window.state::<Client>();
    println!("POST request received at /shell with params: {}", params);

    let backend_url = load_backend_url();
    let full_url = format!("{}api/v1/shell", backend_url);
    
    let result: Result<Value, String> = post_request(client.inner(), &full_url, &params).await;

    match result {
        Ok(parsed_json) => {
            // No need for serde_json::from_str anymore!
            println!("Received response: {}", parsed_json);
            Ok(parsed_json)
        },
        Err(e) => {
            eprintln!("Error during POST request: {}", e);
            Err(e) 
        }
    }
}