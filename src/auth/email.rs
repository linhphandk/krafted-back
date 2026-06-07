use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{info, instrument, warn};

use crate::shared::errors::{AppError, AppResult};

#[async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send_password_reset(&self, to: &str, reset_url: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SmtpEmailProvider {
    mailer: SmtpTransport,
    from: String,
    from_name: String,
}

impl SmtpEmailProvider {
    pub fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        from_email: &str,
        from_name: &str,
    ) -> Self {
        let mailer = if user.is_empty() && password.is_empty() {
            SmtpTransport::builder_dangerous(host)
                .port(port)
                .tls(Tls::None)
                .build()
        } else {
            SmtpTransport::relay(host)
                .unwrap()
                .port(port)
                .credentials(Credentials::new(user.to_string(), password.to_string()))
                .build()
        };

        Self {
            mailer,
            from: from_email.to_string(),
            from_name: from_name.to_string(),
        }
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    #[instrument(skip(self), fields(to = %to))]
    async fn send_password_reset(&self, to: &str, reset_url: &str) -> AppResult<()> {
        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from).parse().unwrap())
            .to(to.parse().unwrap())
            .subject("Reset your password")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Click the link below to reset your password:\n\n{}\n\nThis link expires in 1 hour.",
                reset_url
            ))
            .map_err(|e| {
                warn!("Failed to build email: {:?}", e);
                AppError::Internal
            })?;

        match self.mailer.send(&email) {
            Ok(_) => {
                info!("Password reset email sent to {}", to);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to send email to {}: {:?}", to, e);
                Err(AppError::Internal)
            }
        }
    }
}