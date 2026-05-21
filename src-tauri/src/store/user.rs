use std::fmt::format;

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use serde_json::Value;

pub fn save_user_info(app: AppHandle, json_info: Value) -> Result<(), String> {
    let store = app.store("user_info.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    store.set("user_info", json_info);
    store.save().map_err(|e| e.to_string())?;

    println!("successfully save user info");
    Ok(())
}

#[tauri::command]
pub fn get_user_info(app: AppHandle) -> Result<Value, String> {
    let store = app.store("user_info.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    let user_info = store.get("user_info")
        .ok_or_else(|| "User info not found in store".to_string())?;
        
    Ok(user_info)
}