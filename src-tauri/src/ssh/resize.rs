use crate::ssh::ssh_engine::SshEngine;
use crate::ssh::ssh_instance::ChannelMessage;

#[tauri::command]
pub fn resize_ssh_pty(
    cols: u32,
    rows: u32,
    ip: String,
    state: tauri::State<'_, SshEngine>,
) -> Result<(), String> {
    let registry = state.0.lock().unwrap();

    if let Some(instance) = registry.get(&ip) {
        instance
            .tx
            .send(ChannelMessage::Resize { cols, rows })
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Session not found".into())
    }
}
