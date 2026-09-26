//! Who is emailed about what, and what the emails say.

use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        ports::{
            inbound::{TicketDetail, UserRef},
            outbound::{EmailOutbox, NewEmail},
        },
        principal::Principal,
    },
    domain::{
        ticket::{Status, TicketChange},
        user::Role,
    },
};

#[derive(Debug, Clone)]
pub struct NotificationSettings {
    /// Base URL of the web app, used for links in emails.
    pub app_url: String,
    /// Domain for generated Message-IDs, e.g. `cs-odds.local`.
    pub mail_domain: String,
}

/// Queues notification emails after ticket changes. Delivery happens in the
/// background, so failures here are logged and never fail the user's action.
pub struct Notifier {
    outbox: Arc<dyn EmailOutbox>,
    settings: NotificationSettings,
}

/// What happened to a ticket, from the service's point of view.
pub enum TicketActivity<'a> {
    Created,
    Updated { changes: &'a [TicketChange] },
    Commented { body: &'a str, internal: bool, changes: &'a [TicketChange] },
}

impl Notifier {
    pub fn new(outbox: Arc<dyn EmailOutbox>, settings: NotificationSettings) -> Self {
        Self { outbox, settings }
    }

    pub async fn notify(&self, actor: &Principal, actor_name: &str, ticket: &TicketDetail, activity: TicketActivity<'_>) {
        let emails = plan(&self.settings, actor, actor_name, ticket, &activity);
        if emails.is_empty() {
            return;
        }
        if let Err(err) = self.outbox.enqueue(&emails).await {
            tracing::error!(error = ?err, ticket = %ticket.summary.number, "could not queue notification emails");
        }
    }
}

// ---------- Rules ----------

/// The emails for one piece of activity. Pure, so the rules are easy to test.
pub fn plan(
    settings: &NotificationSettings,
    actor: &Principal,
    actor_name: &str,
    t: &TicketDetail,
    activity: &TicketActivity<'_>,
) -> Vec<NewEmail> {
    let r = Renderer { settings, t };
    let staff_actor = actor.role.is_staff();
    let requester = &t.summary.requester;
    let assignee = t.summary.assignee.as_ref().filter(|a| a.id != actor.user_id);
    let mut out = Vec::new();

    let status_to = |changes: &[TicketChange]| {
        changes.iter().find_map(|c| match c {
            TicketChange::Status { to, .. } => Some(*to),
            _ => None,
        })
    };
    let newly_assigned = |changes: &[TicketChange]| {
        changes.iter().any(|c| matches!(c, TicketChange::Assignee { to: Some(to), .. } if *to != actor.user_id))
    };

    match activity {
        TicketActivity::Created => {
            if staff_actor {
                out.push(r.opened_for_customer(requester, actor_name));
                if let Some(a) = assignee {
                    out.push(r.assigned(a, actor_name));
                }
            } else {
                out.push(r.confirmation(requester));
            }
        }
        TicketActivity::Updated { changes } => {
            if staff_actor && status_to(changes) == Some(Status::Solved) {
                out.push(r.solved(requester, actor_name));
            }
            if let Some(a) = assignee.filter(|_| newly_assigned(changes)) {
                out.push(r.assigned(a, actor_name));
            }
        }
        TicketActivity::Commented { body, internal, changes } => {
            if staff_actor && !internal {
                out.push(r.staff_reply(requester, actor_name, body, status_to(changes)));
            }
            if let Some(a) = assignee {
                if *internal {
                    out.push(r.note(a, actor_name, body));
                } else if !staff_actor {
                    out.push(r.customer_reply(a, actor_name, body));
                }
            }
        }
    }
    out
}

// ---------- Templates ----------

enum Block {
    Para(String),
    /// Someone's message, shown as a quote.
    Quote(String),
}

struct Renderer<'a> {
    settings: &'a NotificationSettings,
    t: &'a TicketDetail,
}

fn first_name(name: &str) -> &str {
    name.split_whitespace().next().unwrap_or(name)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl Renderer<'_> {
    fn number(&self) -> String {
        self.t.summary.number.to_string()
    }

    /// `[#TKT-12] Subject`: the token lets replies by email find the ticket.
    fn subject(&self, prefix: &str) -> String {
        format!("{prefix}[#{}] {}", self.number(), self.t.summary.subject)
    }

    fn link(&self, to: &UserRef) -> String {
        let base = self.settings.app_url.trim_end_matches('/');
        let n = self.t.summary.number.0;
        if to.role == Role::Customer { format!("{base}/portal/requests/{n}") } else { format!("{base}/agent/tickets/{n}") }
    }

    fn email(&self, to: &UserRef, subject: String, blocks: Vec<Block>, button: &str) -> NewEmail {
        let link = self.link(to);
        let greeting = format!("Hi {},", first_name(&to.name));
        let footer = if to.role == Role::Customer {
            "You're receiving this because you contacted CS-ODDS Support."
        } else {
            "You're receiving this as a member of the CS-ODDS support team."
        };

        let mut text = format!("{greeting}\n\n");
        let mut html = format!(
            "<div style=\"font-family:system-ui,-apple-system,'Segoe UI',sans-serif;font-size:15px;line-height:1.55;color:#15171C;max-width:560px\">\
             <p>{}</p>",
            escape(&greeting)
        );
        for block in &blocks {
            match block {
                Block::Para(p) => {
                    text.push_str(p);
                    text.push_str("\n\n");
                    html.push_str(&format!("<p>{}</p>", escape(p)));
                }
                Block::Quote(q) => {
                    for line in q.lines() {
                        text.push_str("> ");
                        text.push_str(line);
                        text.push('\n');
                    }
                    text.push('\n');
                    html.push_str(&format!(
                        "<blockquote style=\"margin:16px 0;padding:12px 16px;background:#F1F2F5;border-radius:8px;white-space:pre-wrap\">{}</blockquote>",
                        escape(q)
                    ));
                }
            }
        }
        text.push_str(&format!("{button}: {link}\n\n— CS-ODDS Support\n\n{footer}\n"));
        html.push_str(&format!(
            "<p style=\"margin:24px 0\"><a href=\"{}\" style=\"background:#3B5BDB;color:#fff;padding:10px 16px;border-radius:8px;text-decoration:none;font-weight:600\">{}</a></p>\
             <p style=\"color:#5F6473\">— CS-ODDS Support</p>\
             <p style=\"color:#8A8FA0;font-size:12px\">{} · {}</p></div>",
            escape(&link),
            escape(button),
            escape(footer),
            escape(&self.number()),
        ));

        NewEmail {
            to_email: to.email.clone(),
            to_name: to.name.clone(),
            subject,
            body_text: text,
            body_html: html,
            message_id: format!("<{}@{}>", Uuid::new_v4(), self.settings.mail_domain),
            ticket_id: Some(self.t.summary.id),
        }
    }

    fn confirmation(&self, to: &UserRef) -> NewEmail {
        self.email(
            to,
            self.subject(""),
            vec![
                Block::Para(format!(
                    "Thanks for contacting us. We've received your request {} and our team will reply as soon as we can.",
                    self.number()
                )),
                Block::Quote(self.t.description.clone()),
                Block::Para("You can follow the conversation and add details at any time.".into()),
            ],
            "View your request",
        )
    }

    fn opened_for_customer(&self, to: &UserRef, actor: &str) -> NewEmail {
        self.email(
            to,
            self.subject(""),
            vec![
                Block::Para(format!("{actor} from our support team opened a request for you, {}:", self.number())),
                Block::Quote(self.t.description.clone()),
                Block::Para("We'll keep you updated here. You can follow the conversation at any time.".into()),
            ],
            "View your request",
        )
    }

    fn staff_reply(&self, to: &UserRef, actor: &str, body: &str, status: Option<Status>) -> NewEmail {
        let mut blocks = vec![Block::Para(format!("{actor} replied to your request:")), Block::Quote(body.to_owned())];
        match status {
            Some(Status::Pending) => blocks.push(Block::Para("We're waiting for your reply before we can continue.".into())),
            Some(Status::Solved) => blocks.push(Block::Para(
                "We've marked this request as solved. If you still need help, reply and we'll reopen it.".into(),
            )),
            _ => {}
        }
        self.email(to, self.subject("Re: "), blocks, "Reply or view the conversation")
    }

    fn solved(&self, to: &UserRef, actor: &str) -> NewEmail {
        self.email(
            to,
            self.subject("Solved: "),
            vec![Block::Para(format!(
                "{actor} marked your request {} as solved. If you still need help, reply and we'll reopen it.",
                self.number()
            ))],
            "View your request",
        )
    }

    fn customer_reply(&self, to: &UserRef, customer: &str, body: &str) -> NewEmail {
        self.email(to, self.subject("Re: "), vec![Block::Para(format!("{customer} replied:")), Block::Quote(body.to_owned())], "Open the ticket")
    }

    fn note(&self, to: &UserRef, actor: &str, body: &str) -> NewEmail {
        self.email(
            to,
            self.subject("Note: "),
            vec![Block::Para(format!("{actor} added an internal note:")), Block::Quote(body.to_owned())],
            "Open the ticket",
        )
    }

    fn assigned(&self, to: &UserRef, actor: &str) -> NewEmail {
        let s = &self.t.summary;
        self.email(
            to,
            self.subject("Assigned to you: "),
            vec![Block::Para(format!(
                "{actor} assigned {} to you.\nCustomer: {}\nPriority: {}",
                self.number(),
                s.requester.name,
                s.priority.as_str()
            ))],
            "Open the ticket",
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::{
        application::ports::inbound::{TicketSummary, TimelineItem},
        domain::{
            ticket::{Channel, Priority, TicketId, TicketNumber},
            user::UserId,
        },
    };

    fn user(name: &str, role: Role) -> UserRef {
        let email = format!("{}@example.com", first_name(name).to_lowercase());
        UserRef { id: UserId(Uuid::new_v4()), name: name.into(), email, role }
    }

    fn settings() -> NotificationSettings {
        NotificationSettings { app_url: "http://localhost:3000/".into(), mail_domain: "cs-odds.test".into() }
    }

    fn ticket(requester: &UserRef, assignee: Option<&UserRef>, author: Option<&UserRef>) -> TicketDetail {
        let timeline = author
            .map(|a| TimelineItem::Comment {
                id: Uuid::new_v4(),
                author: a.clone(),
                body: "hello".into(),
                internal: false,
                attachments: vec![],
                created_at: Utc::now(),
            })
            .into_iter()
            .collect();
        TicketDetail {
            summary: TicketSummary {
                id: TicketId(Uuid::new_v4()),
                number: TicketNumber(12),
                subject: "Password reset link expired".into(),
                status: Status::Open,
                priority: Priority::High,
                channel: Channel::Portal,
                requester: requester.clone(),
                assignee: assignee.cloned(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            description: "Every link says expired. <script>alert(1)</script>".into(),
            attachments: vec![],
            resolved_at: None,
            timeline,
            requester_ticket_count: 1,
        }
    }

    fn as_principal(u: &UserRef) -> Principal {
        Principal { user_id: u.id, role: u.role }
    }

    fn recipients(emails: &[NewEmail]) -> Vec<(&str, &str)> {
        emails.iter().map(|e| (e.to_email.as_str(), e.subject.as_str())).collect()
    }

    #[test]
    fn customer_request_gets_a_confirmation() {
        let jordan = user("Jordan Blake", Role::Customer);
        let t = ticket(&jordan, None, None);
        let emails = plan(&settings(), &as_principal(&jordan), &jordan.name, &t, &TicketActivity::Created);
        assert_eq!(recipients(&emails), [("jordan@example.com", "[#TKT-12] Password reset link expired")]);
        let e = &emails[0];
        assert!(e.body_text.starts_with("Hi Jordan,"));
        assert!(e.body_text.contains("http://localhost:3000/portal/requests/12"));
        assert!(e.message_id.starts_with('<') && e.message_id.ends_with("@cs-odds.test>"));
        // User text is escaped in HTML.
        assert!(e.body_html.contains("&lt;script&gt;") && !e.body_html.contains("<script>"));
    }

    #[test]
    fn staff_created_ticket_emails_customer_and_other_assignee() {
        let jordan = user("Jordan Blake", Role::Customer);
        let maya = user("Maya Chen", Role::Agent);
        let daniel = user("Daniel Okafor", Role::Agent);
        let t = ticket(&jordan, Some(&daniel), None);
        let emails = plan(&settings(), &as_principal(&maya), &maya.name, &t, &TicketActivity::Created);
        assert_eq!(recipients(&emails), [
            ("jordan@example.com", "[#TKT-12] Password reset link expired"),
            ("daniel@example.com", "Assigned to you: [#TKT-12] Password reset link expired"),
        ]);
        assert!(emails[1].body_text.contains("http://localhost:3000/agent/tickets/12"));

        // Assigning to yourself doesn't email you.
        let mine = ticket(&jordan, Some(&maya), None);
        assert_eq!(plan(&settings(), &as_principal(&maya), &maya.name, &mine, &TicketActivity::Created).len(), 1);
    }

    #[test]
    fn staff_reply_reaches_customer_with_status_note() {
        let jordan = user("Jordan Blake", Role::Customer);
        let maya = user("Maya Chen", Role::Agent);
        let t = ticket(&jordan, Some(&maya), Some(&maya));
        let changes = [TicketChange::Status { from: Status::Open, to: Status::Pending }];
        let activity = TicketActivity::Commented { body: "Try this link", internal: false, changes: &changes };
        let emails = plan(&settings(), &as_principal(&maya), &maya.name, &t, &activity);
        assert_eq!(recipients(&emails), [("jordan@example.com", "Re: [#TKT-12] Password reset link expired")]);
        assert!(emails[0].body_text.contains("Maya Chen replied"));
        assert!(emails[0].body_text.contains("> Try this link"));
        assert!(emails[0].body_text.contains("waiting for your reply"));
    }

    #[test]
    fn internal_notes_never_reach_the_customer() {
        let jordan = user("Jordan Blake", Role::Customer);
        let maya = user("Maya Chen", Role::Agent);
        let daniel = user("Daniel Okafor", Role::Agent);
        let t = ticket(&jordan, Some(&maya), Some(&daniel));
        let activity = TicketActivity::Commented { body: "Known Outlook issue", internal: true, changes: &[] };
        let emails = plan(&settings(), &as_principal(&daniel), &daniel.name, &t, &activity);
        assert_eq!(recipients(&emails), [("maya@example.com", "Note: [#TKT-12] Password reset link expired")]);
        // The assignee's own note emails nobody.
        assert!(plan(&settings(), &as_principal(&maya), &maya.name, &t, &activity).is_empty());
    }

    #[test]
    fn customer_reply_goes_to_the_assignee_only() {
        let jordan = user("Jordan Blake", Role::Customer);
        let maya = user("Maya Chen", Role::Agent);
        let reply = TicketActivity::Commented { body: "Still broken", internal: false, changes: &[] };
        let assigned = ticket(&jordan, Some(&maya), Some(&jordan));
        let emails = plan(&settings(), &as_principal(&jordan), &jordan.name, &assigned, &reply);
        assert_eq!(recipients(&emails), [("maya@example.com", "Re: [#TKT-12] Password reset link expired")]);
        assert!(emails[0].body_text.contains("Jordan Blake replied"));

        let unassigned = ticket(&jordan, None, Some(&jordan));
        assert!(plan(&settings(), &as_principal(&jordan), &jordan.name, &unassigned, &reply).is_empty());
    }

    #[test]
    fn updates_email_on_solve_by_staff_and_on_assignment() {
        let jordan = user("Jordan Blake", Role::Customer);
        let maya = user("Maya Chen", Role::Agent);
        let daniel = user("Daniel Okafor", Role::Agent);
        let t = ticket(&jordan, Some(&daniel), None);
        let changes = [
            TicketChange::Status { from: Status::Open, to: Status::Solved },
            TicketChange::Assignee { from: None, to: Some(daniel.id) },
        ];
        let emails = plan(&settings(), &as_principal(&maya), &maya.name, &t, &TicketActivity::Updated { changes: &changes });
        assert_eq!(recipients(&emails), [
            ("jordan@example.com", "Solved: [#TKT-12] Password reset link expired"),
            ("daniel@example.com", "Assigned to you: [#TKT-12] Password reset link expired"),
        ]);

        // A customer solving their own request, or a priority change, sends nothing.
        let solved = [TicketChange::Status { from: Status::Open, to: Status::Solved }];
        assert!(plan(&settings(), &as_principal(&jordan), &jordan.name, &t, &TicketActivity::Updated { changes: &solved }).is_empty());
        let prio = [TicketChange::Priority { from: Priority::High, to: Priority::Urgent }];
        assert!(plan(&settings(), &as_principal(&maya), &maya.name, &t, &TicketActivity::Updated { changes: &prio }).is_empty());
    }
}
