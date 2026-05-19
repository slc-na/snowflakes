use crate::ssh::ssh_instance::SshInstance;
use ssh2::{Channel, Stream};
use std::io::Read;
use std::io::Write;
use std::time::Duration;
use std::time::Instant;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::Emitter;
use tauri::Window;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;

pub struct SshEngine(pub Arc<Mutex<HashMap<String, SshInstance>>>);

impl SshEngine {
    pub fn spawn_thread_write(
        rx: UnboundedReceiver<String>,
        channel: Channel,
        stop_rx: watch::Receiver<bool>,
    ) {
        tauri::async_runtime::spawn_blocking(move || {
            let mut rx_mut = rx;
            let mut channel_mut = channel;
            while let Some(input) = rx_mut.blocking_recv() {
                if *stop_rx.borrow() {
                    break;
                }
                if channel_mut.write_all(input.as_bytes()).is_err() {
                    break;
                }
                if channel_mut.flush().is_err() {
                    break;
                }
            }
            println!("Writer thread exited");
        });
    }
    pub fn spawn_thread_read(
        session_key: String,
        reader: Stream,
        window_clone: Window,
        stop_rx: watch::Receiver<bool>,
    ) {
        tauri::async_runtime::spawn_blocking(move || {
            let mut buffer = [0u8; 4096];
            println!("Reader thread started for {}", session_key.clone());
            let mut reader_mut = reader;
            let mut acc = String::new();
            let mut last_emit = Instant::now();
            loop {
                if *stop_rx.borrow() {
                    break;
                }
                match reader_mut.read(&mut buffer) {
                    Ok(0) => {
                        println!("Stream closed");
                        break;
                    } // Stream tertutup
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        acc.push_str(&output);
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            std::thread::sleep(Duration::from_millis(50));

                            continue; // Coba baca lagi di iterasi berikutnya
                        } else {
                            println!("Reader error kritis: {:?}", e);
                            window_clone
                                .emit(
                                    &format!("ssh-error-output-{}", session_key),
                                    format!("Error: {}", e),
                                )
                                .ok();
                            break;
                        }
                    }
                }

                if last_emit.elapsed() > Duration::from_millis(50) {
                    if !acc.is_empty() {
                        window_clone
                            .emit(&format!("ssh-output-{}", session_key), acc.clone())
                            .ok();
                        acc.clear();
                    }
                    last_emit = Instant::now();
                }
            }

            println!("Thread Exited, Cleaning up session");
        });
    }
}
