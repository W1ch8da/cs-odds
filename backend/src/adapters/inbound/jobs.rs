//! Timer-driven adapters: background work that calls inbound ports on a
//! schedule, the way HTTP handlers call them on requests.

use std::{sync::Arc, time::Duration};

use tokio::{task::JoinHandle, time::MissedTickBehavior};

use crate::application::ports::inbound::DeliverEmails;

/// Sends queued email every `every`, forever. Errors are logged and the next
/// tick tries again.
pub fn spawn_email_worker(delivery: Arc<dyn DeliverEmails>, every: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(every);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            match delivery.deliver_due().await {
                Ok(report) if report.sent + report.retrying + report.failed > 0 => {
                    tracing::info!(sent = report.sent, retrying = report.retrying, failed = report.failed, "email delivery");
                }
                Ok(_) => {}
                Err(err) => tracing::error!(error = ?err, "email worker error"),
            }
        }
    })
}
