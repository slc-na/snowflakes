use ssh2::Channel;
use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};

// A wrong target host/port makes the bastion's jump script close the channel
// almost immediately, while a real shell session stays open. The ssh2
// handshake/auth to the *bastion* already succeeded in that case, so the only
// remaining signal is how long the channel survives after exec().
const FAST_EXIT_GRACE: Duration = Duration::from_millis(1500);
const FAST_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct SshInstance {
    pub tx: mpsc::UnboundedSender<String>,
    pub stop_tx: watch::Sender<bool>
}

impl SshInstance {
    pub fn bastion_session(
        bastion: String,
        initial_password: String,
        initial_username: String,
        params: HashMap<String, String>,
    ) -> Result<(Channel, Vec<u8>), String> {
        let tcp = std::net::TcpStream::connect(format!("{}:22", bastion))
            .map_err(|e| format!("Gagal koneksi ke server: {}", e))?;

        let mut sess = ssh2::Session::new().map_err(|e| e.to_string())?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| format!("Handshake gagal: {}", e.message()))?;

        sess.userauth_password(&initial_username, &initial_password)
            .map_err(|e| format!("Login gagal: {}; Username : {}; Password length : {}", e.message(), &initial_username, &initial_password.len()))?;
        let (cols, rows) = term_size::dimensions().unwrap_or((220, 50));
        let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
        channel
            .request_pty(
                "xterm-256color",
                None,
                Some((cols as u32, rows as u32, 0, 0)),
            )
            .map_err(|e| e.to_string())?;

        // equivalent of: ssh -t {initial_username}@{bastion} <params...>
        // the bastion's shell intercepts the "key=value" pairs to know which
        // downstream host/port (and any other template-defined params) to jump to.
        let mut payload = String::new();
        for (key, value) in params.iter() {
            payload.push(' ');
            payload.push_str(key);
            payload.push('=');
            payload.push_str(value);
        }
        let payload = payload.trim_start();

        println!("Executing command on bastion: {}", payload);
        channel.exec(payload).map_err(|e| e.to_string())?;

        // `Channel::eof()` only reflects packets libssh2 has already processed,
        // so it never flips to true on its own — we have to actually attempt
        // reads to pump the session and let it notice the remote's EOF/close.
        // Use a non-blocking session for this probe so a legitimate, quiet
        // shell doesn't hang here; any bytes we do read (e.g. a banner) are
        // captured in `early_output` so the caller can forward them instead of
        // losing them.
        sess.set_blocking(false);

        let started = Instant::now();
        let mut closed_early = false;
        let mut early_output = Vec::new();
        let mut buf = [0u8; 4096];
        while started.elapsed() < FAST_EXIT_GRACE {
            match channel.read(&mut buf) {
                Ok(0) => {
                    closed_early = true;
                    break;
                }
                Ok(n) => {
                    early_output.extend_from_slice(&buf[..n]);
                    if channel.eof() {
                        closed_early = true;
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if channel.eof() {
                        closed_early = true;
                        break;
                    }
                    std::thread::sleep(FAST_EXIT_POLL_INTERVAL);
                }
                Err(_) => {
                    closed_early = true;
                    break;
                }
            }
        }

        if closed_early {
            sess.set_blocking(true);
            let _ = channel.close();
            let _ = channel.wait_close();
            let exit_code = channel.exit_status().unwrap_or(-1);
            let detail = String::from_utf8_lossy(&early_output).trim().to_string();
            println!(
                "Target rejected the connection immediately (exit code {}): {}",
                exit_code, detail
            );

            // "FAST_EXIT::" is a stable marker the frontend matches on to show
            // a friendly toast instead of this raw diagnostic string.
            return Err(if detail.is_empty() {
                format!(
                    "FAST_EXIT::Target rejected the connection immediately (exit code {}). Check the target host/port.",
                    exit_code
                )
            } else {
                format!(
                    "FAST_EXIT::Target rejected the connection immediately (exit code {}): {}",
                    exit_code, detail
                )
            });
        }

        sess.set_blocking(true);
        channel
            .request_pty_size(cols as u32, rows as u32, Some(0), Some(0))
            .map_err(|e| e.to_string())?;
        sess.set_blocking(false);
        Ok((channel, early_output))
    }
}
