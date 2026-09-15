use crate::ssh::ssh_engine::SshEngine;
use crate::ssh::ssh_instance::ChannelMessage;

#[tauri::command]
pub fn send_ssh_input(
    input: String,
    ip: String,
    state: tauri::State<'_, SshEngine>,
) -> Result<(), String> {
    let registry = state.0.lock().unwrap();

    if let Some(instance) = registry.get(&ip) {
        println!("Sending input: {}", input);
        instance
            .tx
            .send(ChannelMessage::Input(input))
            .map_err(|e| println!("Send SSh Input fail : {}", e.to_string()));
        Ok(())
    } else {
        Err("Session not found".into())
    }
}
