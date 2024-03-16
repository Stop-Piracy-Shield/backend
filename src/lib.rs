pub mod models;
pub mod schema;

use std::env::VarError;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;

use lettre::{Message, SmtpTransport, Transport};
use chrono::NaiveDateTime;

struct EmailConfiguration {
    from: String,
    url: String,
    username: String,
    password: String,
    website_url: String
}

impl EmailConfiguration {
    fn read_env() -> Result<Self, VarError> {
        Ok(Self {
            from: std::env::var("SMTP_FROM")?,
            url: std::env::var("SMTP_URL")?,
            username: std::env::var("SMTP_USERNAME")?,
            password: std::env::var("SMTP_PASSWORD")?,
            website_url: std::env::var("WEBSITE_URL")?,
        })
    }
}

fn build_email(
    signature: &models::Signature,
    config: &EmailConfiguration,
    subject: String,
    body: String
) -> Result<Message, uuid::Uuid> {
    Message::builder()
        .from(
            config
                .from
                .parse()
                .expect("Error parsing SMTP_FROM address"),
        )
        .to(format!(
            "{} {} <{}>",
            signature.first_name, signature.last_name, signature.email
        )
        .parse()
        .map_err(|_| signature.id)?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(body)
        .map_err(|_| signature.id)
}

fn send_email(
    config: EmailConfiguration,
    signature: models::Signature,
    subject: String,
    body: String
) -> Result<(), uuid::Uuid> {
    let email = build_email(&signature, &config, subject, body)?;

    let creds = Credentials::new(config.username, config.password);
    let mailer = SmtpTransport::from_url(&config.url)
    .map_err(|_| signature.id)?
    .credentials(creds)
    .build();

    mailer
    .send(&email)
    .map_err(|_| signature.id)
    .map(|_| ())
}

pub fn send_confirmation_email(
    signature: models::Signature,
) -> Result<(), uuid::Uuid> {
    let config = EmailConfiguration::read_env().expect("Error reading SMTP configuration");
    let validation_url = format!("{}/verifica-email/{}", config.website_url, generate_auth_token(&signature));

    let body = [
        "<h1>Lettera aperta contro gli eccessi di Piracy Shield</h1>",
        format!("Ciao {} {},", signature.first_name, signature.last_name).as_str(),
        " premi sul link sotto per <b>verificare la tua firma</b>.",
        format!("<br/><a href=\"{}\">{}</a>", validation_url, validation_url).as_str(),
    ]
    .concat();

    return send_email(config, signature, "Verifica la firma. Lettera aperta contro gli eccessi di Piracy Shield".into(), body)
}

pub fn send_sign_email(
    signature: models::Signature,
) -> Result<(), uuid::Uuid> {
    let config = EmailConfiguration::read_env().expect("Error reading SMTP configuration");
    let revoke_url = format!("{}/revoca-email/{}", config.website_url, generate_auth_token(&signature));

    let body = [
        "<h1>Lettera aperta contro gli eccessi di Piracy Shield</h1>",
        format!("Ciao {} {},", signature.first_name, signature.last_name).as_str(),
        " ti confermiamo che la tua firma è stata <b>registrata correttamente<b>.",
        "<br><br><br><br><br>",
        "Se per qualsiasi motivo desideri <b>rimuovere</b> la tua firma puoi premere il link sotto",
        format!("<br/><a href=\"{}\">{}</a>", revoke_url, revoke_url).as_str(),
    ]
    .concat();

    return send_email(config, signature, "Conferma firma. Lettera aperta contro gli eccessi di Piracy Shield".into(), body)
}

pub fn generate_auth_token(signature: &models::Signature) -> String {
    let data: NaiveDateTime;
    if signature.verified {
        data = signature.verified_at.unwrap();
    } else {
        data = signature.created_at;
    }

    return signature.id.to_string() + &sha256::digest(
        signature.id.to_string()
        + &data.and_utc().timestamp_nanos_opt().unwrap().to_string()
        + &signature.email
    );
}
