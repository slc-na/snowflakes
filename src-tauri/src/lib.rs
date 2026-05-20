// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use crate::sftp::sftp_engine::SftpEngine;
use crate::ssh::ssh_engine::SshEngine;
use reqwest::Client;
use serde::Serialize;
use ssh::input::send_ssh_input;
use ssh::manage_session::disconnect;
use ssh::manage_session::get_active_session;
use ssh::reconnect::reconnect_to_session;
use ssh::start::start_ssh_session;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use sysinfo::System;

mod oauth;
mod sftp;
mod ssh;

#[derive(Serialize)]
struct SystemStats {
    cpu_usage: u32,
    ram_usage: u32,
}

struct MetricsState(Mutex<System>);
#[tauri::command]
fn get_system_stats(state: tauri::State<'_, MetricsState>) -> SystemStats {
    let mut sys = state.0.lock().unwrap();
    sys.refresh_cpu();
    sys.refresh_memory();

    let cpu_usage = sys.global_cpu_info().cpu_usage() as u32;
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let ram_usage = ((used_mem as f64 / total_mem as f64) * 100.0) as u32;

    SystemStats {
        cpu_usage,
        ram_usage,
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// #[tauri::command]
// fn submit_ssh_password(
//     input_state: tauri::State<'_, SshInputState>,
//     password: String,
// ) -> Result<(), String> {
//     let sender_lock = input_state.0.lock().unwrap();
//     if let Some(tx) = sender_lock.as_ref() {
//         tx.blocking_send(password).map_err(|e| e.to_string())?;
//         Ok(())
//     } else {
//         Err("No active SSH session".into())
//     }
// }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let metric_state = MetricsState(Mutex::new(System::new()));
    let ssh_state = SshEngine(Arc::new(Mutex::new(HashMap::new())));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_stronghold::Builder::new(|password| {
                use argon2::{self, hash_raw, Config, Variant, Version};

                let config = Config {
                    lanes: 4,
                    mem_cost: 10_000,
                    time_cost: 10,
                    variant: Variant::Argon2id,
                    version: Version::Version13,
                    ..Default::default()
                };
                let salt = "your-salt".as_bytes();
                let key =
                    hash_raw(password.as_ref(), salt, &config).expect("failed to hash password");

                key.to_vec()
            })
            .build(),
        )
        .setup(|app| {
            // Mengambil handle untuk digunakan di dalam closure atau thread
            let _app_handle = app.handle();

            // Contoh: Jika kamu ingin melakukan sesuatu saat app baru nyala
            // app_handle.emit_all("sys-status", "Backend Ready").unwrap();

            Ok(())
        })
        .manage(metric_state)
        .manage(ssh_state)
        .manage(Client::new())
        .manage(SftpEngine(Arc::new(Mutex::new(HashMap::new()))))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_oauth::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_system_stats,
            start_ssh_session,
            send_ssh_input,
            disconnect,
            get_active_session,
            reconnect_to_session,
            sftp::start::start_sftp_session,
            sftp::manage_session::get_active_sftp_session,
            sftp::manage_session::disconnect_sftp,
            sftp::list::sftp_list_dir,
            sftp::download::sftp_download_file,
            sftp::upload::sftp_upload_file,
            oauth::server::start_server,
            oauth::token::get_token,
            oauth::token::delete_token,
            oauth::auth::open_oauth_login,
            oauth::auth::oauth_is_authenticated
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
