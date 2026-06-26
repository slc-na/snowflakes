use std::collections::HashMap;
use tauri::Emitter;
use tokio::sync::{mpsc, watch};

use crate::ssh::{
    ssh_engine::SshEngine,
    ssh_instance::{self, SshInstance},
};

#[tauri::command]
pub async fn reconnect_to_session(
    state: tauri::State<'_, SshEngine>,
    key: String,
    window: tauri::Window,
    hostname: String,
    bastion: String,
    initial_password: String,
    initial_username: String,
    params: HashMap<String, String>,
) -> Result<(), String> {
    let registry = state.0.lock().unwrap();

    if registry.contains_key(&key) {
        let _ = window.emit("session_updated", key.clone());
        return Ok(());
    }

    drop(registry);

    let (channel, early_output) = ssh_instance::SshInstance::bastion_session(
        bastion,
        initial_password,
        initial_username,
        params,
    )
    .map_err(|e| e.to_string())?;

    let reader = channel.stream(0);

    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let (stop_tx, stop_rx) = watch::channel(false);

    let mut registry = state.0.lock().unwrap();

    if registry.contains_key(&key) {
        let _ = window.emit("session_updated", key.clone());
        return Ok(());
    }

    registry.insert(key.clone(), SshInstance { tx, stop_tx });

    let window_clone = window.clone();

    if !early_output.is_empty() {
        let _ = window.emit(
            &format!("ssh-output-{}", key),
            String::from_utf8_lossy(&early_output).to_string(),
        );
    }
    SshEngine::spawn_thread_write(rx, channel, stop_rx.clone());
    SshEngine::spawn_thread_read(key.clone(), reader, window_clone, stop_rx.clone());
    let _ = window.emit("session_updated", key.clone());
    Ok(())
}
