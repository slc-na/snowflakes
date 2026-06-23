use tauri::Emitter;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::ssh::ssh_instance;
use crate::ssh::{ssh_engine::SshEngine, ssh_instance::SshInstance};


// works untuk duplicate juga
#[tauri::command]
pub async fn start_ssh_session(
    window: tauri::Window,
    state: tauri::State<'_, SshEngine>,
    hostname: String,
    bastion: String,
    initial_password: String,
    initial_username: String,
    port: u16,
) -> Result<String, String> {
    let channel = ssh_instance::SshInstance::bastion_session(
        hostname.clone(),
        bastion,
        initial_password,
        initial_username,
        port,
    )?;
    let mut registry = state.0.lock().unwrap();
    let hostname_clone = hostname.clone();
    let reader = channel.stream(0);
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let (stop_tx, stop_rx) = watch::channel(false);
    let session_key = format!(
        "{}_{}",
        hostname_clone.clone().replace(".", "-"),
        Uuid::new_v4().to_string()
    );
    registry.insert(session_key.clone(), SshInstance { 
        tx : tx, 
        stop_tx : stop_tx
    });
    let _ = window.emit("session_updated", session_key.clone());
    let window_clone = window.clone();
    SshEngine::spawn_thread_write(rx, channel, stop_rx.clone());
    SshEngine::spawn_thread_read(session_key.clone(), reader, window_clone, stop_rx.clone());

    Ok(session_key)
}
