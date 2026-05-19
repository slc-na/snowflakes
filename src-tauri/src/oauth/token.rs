use keyring::Entry;


pub fn save_access_token(token: &str) -> Result<(), String> {

    // Create an entry in the OS Keyring (Service Name, Account Name)
    let entry = Entry::new("snow-flakes", "access_token")
        .map_err(|e| e.to_string())?;

    // save the token 
    entry.set_password(&token).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_token() -> Result<String, String> {
    let entry = Entry::new("snow-flakes", "access_token")
        .map_err(|e| e.to_string())?;
        
    let token = entry.get_password().map_err(|e| e.to_string())?;
    
    Ok(token)
}

#[tauri::command]
pub async fn delete_token() -> Result<(), String> {
    let entry = Entry::new("snow-flakes", "access_token")
        .map_err(|e| e.to_string())?;
        
    entry.delete_credential().map_err(|e| e.to_string())?;
    
    Ok(())
}