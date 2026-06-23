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
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

mod oauth;
mod sftp;
mod ssh;
mod store;
mod config;
mod http;

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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update(handle.clone()).await {
                    let _ = handle
                        .dialog()
                        .message(format!("Update Error: {}", e))
                        .title("Updater Debug")
                        .blocking_show();
                }
            });
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
            oauth::auth::oauth_is_authenticated,
            store::user::get_user_info,
            http::server::get_server,
            http::server::update_server,
            http::shell::get_shell,
            http::shell::update_shell,
            http::shell::insert_shell,
            http::server::get_bastion_ip,
            http::guacamole::guacamole_login,
            http::guacamole::open_guacamole_window,
            http::guacamole::close_guacamole_window,
            http::guacamole::focus_guacamole_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;
        // 1. Show a confirmation dialog
        let update_version = update.version.clone();
        let yes = app
            .dialog()
            .message(format!(
                "A new version ({}) is available. Would you like to install it now?",
                update_version
            ))
            .title("Update Available")
            .buttons(MessageDialogButtons::YesNo)
            .blocking_show();
        if yes {
            update
                .download_and_install(
                    |chunk_length, content_length| {
                        downloaded += chunk_length;
                        // You could emit an event here to show progress in your CSS/JS UI
                    },
                    || {
                        println!("download finished");
                    },
                )
                .await?;
            println!("update installed");
            app.restart();
        }
    }
    Ok(())
}
