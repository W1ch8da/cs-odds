//! SMTP delivery with lettre. The only place that knows about SMTP.

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
};

use crate::application::{
    error::AppResult,
    ports::outbound::{MailSender, NewEmail},
};

pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpSender {
    /// `url` examples: `smtp://localhost:1025` (Mailpit, no TLS),
    /// `smtps://user:pass@smtp.example.com` (TLS),
    /// `smtp://user:pass@smtp.example.com:587?tls=required` (STARTTLS).
    pub fn new(url: &str, from: &str) -> anyhow::Result<Self> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            .context("SMTP_URL is not a valid SMTP URL")?
            .build();
        let from = from.parse().with_context(|| format!("MAIL_FROM is not a valid address: {from}"))?;
        Ok(Self { transport, from })
    }

    /// The domain of the sender address, for Message-IDs.
    pub fn from_domain(&self) -> String {
        self.from.email.domain().to_owned()
    }
}

#[async_trait]
impl MailSender for SmtpSender {
    async fn send(&self, e: &NewEmail) -> AppResult<()> {
        let to = Mailbox::new(Some(e.to_name.clone()), e.to_email.parse().context("invalid recipient address")?);
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(&e.subject)
            .message_id(Some(e.message_id.clone()))
            .multipart(MultiPart::alternative_plain_html(e.body_text.clone(), e.body_html.clone()))
            .context("could not build email")?;
        self.transport.send(message).await.map_err(|err| anyhow!("SMTP send failed: {err}"))?;
        Ok(())
    }
}
