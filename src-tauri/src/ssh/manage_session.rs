use tauri::Emitter;

use crate::ssh::{ssh_engine::SshEngine};

#[tauri::command]
pub async fn get_active_session(
    state: tauri::State<'_, SshEngine>,
) -> Result<Vec<String>, String> {
    let registry = state.0.lock().unwrap();

    let keys = registry.keys().cloned().collect::<Vec<String>>();
    Ok(keys)
}

#[tauri::command]
pub async fn disconnect(
    window: tauri::Window,
    session_key: String,
    state: tauri::State<'_, SshEngine>,
) -> Result<(), String> {
    let mut registry = state.0.lock().unwrap();

    if let Some(session) = registry.remove(&session_key) {
        // kirim signal stop ke thread
        let _ = session.stop_tx.send(true);

        println!("Session {} disconnected", session_key);
        window
            .emit("session_updated", session_key)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Session {} tidak ditemukan", session_key))
    }
}
