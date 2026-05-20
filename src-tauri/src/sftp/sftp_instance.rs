use ssh2::Sftp;
use tokio::sync::watch;

pub struct SftpInstance {
    pub sftp: Sftp,
    pub stop_tx: watch::Sender<bool>,
}

impl SftpInstance {
    pub fn session(
        hostname: String,
        initial_password: String,
        initial_username: String,
    ) -> Result<Sftp, String> {
        let tcp = std::net::TcpStream::connect(format!("{}:22", hostname))
            .map_err(|e| format!("Gagal koneksi ke host: {}", e))?;

        let mut sess = ssh2::Session::new().map_err(|e| e.to_string())?;

        sess.set_tcp_stream(tcp);

        sess.handshake()
            .map_err(|e| format!("Handshake gagal: {}", e.message()))?;

        sess.userauth_password(&initial_username, &initial_password)
            .map_err(|e| format!("Login gagal: {}", e.message()))?;

        if !sess.authenticated() {
            return Err("Autentikasi gagal".into());
        }

        let sftp = sess
            .sftp()
            .map_err(|e| format!("Gagal membuka SFTP: {}", e))?;

        Ok(sftp)
    }
}
