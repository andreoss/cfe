use app::{Mailer, Message, TokenDigest};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

pub struct Sha256Digest;

impl TokenDigest for Sha256Digest {
    fn digest(&self, secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

pub fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct LogMailer {
    path: String,
}

impl LogMailer {
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[async_trait::async_trait]
impl Mailer for LogMailer {
    async fn send(&self, message: &Message) {
        let line = format!(
            "to={}\nsubject={}\nbody={}\n---\n",
            message.to, message.subject, message.body
        );
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .expect("open mail log");
            file.write_all(line.as_bytes()).expect("write mail log");
        })
        .await
        .expect("mail log task");
    }
}

pub struct SmtpMailer {
    url: String,
    from: String,
}

impl SmtpMailer {
    pub fn new(url: String, from: String) -> Self {
        Self { url, from }
    }
}

#[async_trait::async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, message: &Message) {
        let url = self.url.clone();
        let from = self.from.clone();
        let to = message.to.clone();
        let subject = message.subject.clone();
        let body = message.body.clone();
        tokio::task::spawn_blocking(move || {
            let payload = format!(
                "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{body}\r\n"
            );
            if let Err(error) = deliver(&url, &from, &to, &payload) {
                eprintln!("mail delivery failed: {error}");
            }
        })
        .await
        .expect("mail task");
    }
}

fn deliver(url: &str, from: &str, to: &str, payload: &str) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;

    let address = url.strip_prefix("smtp://").unwrap_or(url);
    let stream = TcpStream::connect(address)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    for command in [
        format!("HELO localhost\r\n"),
        format!("MAIL FROM:<{from}>\r\n"),
        format!("RCPT TO:<{to}>\r\n"),
        "DATA\r\n".to_owned(),
    ] {
        writer.write_all(command.as_bytes())?;
        line.clear();
        reader.read_line(&mut line)?;
    }
    writer.write_all(payload.as_bytes())?;
    writer.write_all(b".\r\n")?;
    line.clear();
    reader.read_line(&mut line)?;
    writer.write_all(b"QUIT\r\n")?;
    Ok(())
}

pub fn build() -> Arc<dyn Mailer + Send + Sync> {
    let transport = std::env::var("MAIL_TRANSPORT").unwrap_or_else(|_| "log".to_owned());
    match transport.as_str() {
        "log" => Arc::new(LogMailer::new(
            std::env::var("MAIL_LOG").unwrap_or_else(|_| "mail.log".to_owned()),
        )),
        "smtp" => Arc::new(SmtpMailer::new(
            std::env::var("SMTP_URL").expect("SMTP_URL must be set for the smtp transport"),
            std::env::var("MAIL_FROM").unwrap_or_else(|_| "forum@localhost".to_owned()),
        )),
        other => panic!("unknown mail transport: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_hides_the_secret_and_is_stable() {
        let digest = Sha256Digest.digest("secret-code");
        assert_ne!(digest, "secret-code");
        assert!(!digest.contains("secret"));
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, Sha256Digest.digest("secret-code"));
        assert_ne!(digest, Sha256Digest.digest("secret-cod"));
    }

    #[test]
    fn a_generated_secret_is_long_and_unpredictable() {
        let first = generate_secret();
        let second = generate_secret();
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn the_log_mailer_appends_each_message() {
        let dir = std::env::temp_dir().join(format!("tcbs-mail-{}", generate_secret()));
        let path = dir.to_string_lossy().to_string();
        let mailer = LogMailer::new(path.clone());
        mailer
            .send(&Message {
                to: "someone@example.com".to_owned(),
                subject: "Reset your password".to_owned(),
                body: "code here".to_owned(),
            })
            .await;
        let written = std::fs::read_to_string(&path).expect("read mail log");
        assert!(written.contains("to=someone@example.com"));
        assert!(written.contains("code here"));
        std::fs::remove_file(&path).ok();
    }
}
