pub mod models;
pub mod schema;

use std::env::VarError;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;

use lettre::{Message, SmtpTransport, Transport};

struct EmailConfiguration {
    from: String,
    url: String,
    username: String,
    password: String,
}

impl EmailConfiguration {
    fn read_env() -> Result<Self, VarError> {
        Ok(Self {
            from: std::env::var("SMTP_FROM")?,
            url: std::env::var("SMTP_URL")?,
            username: std::env::var("SMTP_USERNAME")?,
            password: std::env::var("SMTP_PASSWORD")?,
        })
    }
}

fn build_email(
    signature: &models::Signature,
    config: &EmailConfiguration,
) -> Result<Message, uuid::Uuid> {
    let validation_url = format!("https://example.com/verifica-email?token={}", signature.id);

    Message::builder()
        .from(config.from.parse().map_err(|_| signature.id)?)
        .to(format!(
            "{} {} <{}>",
            signature.first_name, signature.last_name, signature.email
        )
        .parse()
        .map_err(|_| signature.id)?)
        .subject("Verifica la firma. Lettera aperta contro gli eccessi di Piracy Shield")
        .header(ContentType::TEXT_HTML)
        .body(
            [
                "<h1>Lettera aperta contro gli eccessi di Piracy Shield</h1>",
                format!("Ciao {} {},", signature.first_name, signature.last_name).as_str(),
                "premi sul link sotto per <b>verificare la tua firma</b>.",
                format!("<br/><a href=\"{}\">{}</a>", validation_url, validation_url).as_str(),
            ]
            .concat(),
        )
        .map_err(|_| signature.id)
}

pub fn send_confirmation_email(signature: models::Signature) -> Result<(), uuid::Uuid> {
    let config = EmailConfiguration::read_env().map_err(|_| signature.id)?;
    let email = build_email(&signature, &config)?;

    let creds = Credentials::new(config.username, config.password);
    let mailer = SmtpTransport::from_url(&config.url)
        .map_err(|_| signature.id)?
        .credentials(creds)
        .build();

    mailer.send(&email).map_err(|_| signature.id).map(|_| ())
}
