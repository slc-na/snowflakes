use tauri::Emitter;
use tokio::sync::watch;
use uuid::Uuid;

use crate::sftp::{
    sftp_engine::SftpEngine,
    sftp_instance::{self, SftpInstance},
};

#[tauri::command]
pub async fn start_sftp_session(
    window: tauri::Window,
    state: tauri::State<'_, SftpEngine>,
    hostname: String,
    initial_password: String,
    initial_username: String,
) -> Result<String, String> {
    let sftp =
        sftp_instance::SftpInstance::session(hostname.clone(), initial_password, initial_username)?;

    let (stop_tx, _stop_rx) = watch::channel(false);

    let session_key = format!(
        "{}_{}",
        hostname.replace(".", "-"),
        Uuid::new_v4().to_string()
    );

    let mut registry = state.0.lock().unwrap();
    registry.insert(session_key.clone(), SftpInstance { sftp, stop_tx });
    drop(registry);

    let _ = window.emit("sftp_session_updated", session_key.clone());

    Ok(session_key)
}
