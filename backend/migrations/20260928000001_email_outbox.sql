-- Outgoing email queue. Requests only insert rows; a background worker
-- delivers them, so a slow or unavailable mail server never affects the app.
CREATE TYPE email_status AS ENUM ('pending', 'sending', 'sent', 'failed');

CREATE TABLE email_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    to_email        TEXT NOT NULL,
    to_name         TEXT NOT NULL,
    subject         TEXT NOT NULL,
    body_text       TEXT NOT NULL,
    body_html       TEXT NOT NULL,
    -- RFC 5322 Message-ID (with angle brackets), used to thread replies.
    message_id      TEXT NOT NULL UNIQUE,
    ticket_id       UUID REFERENCES tickets (id) ON DELETE SET NULL,
    status          email_status NOT NULL DEFAULT 'pending',
    attempts        INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A 'sending' row whose lock expired was abandoned by a crashed worker.
    locked_until    TIMESTAMPTZ,
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at         TIMESTAMPTZ
);

CREATE INDEX email_outbox_due_idx ON email_outbox (next_attempt_at) WHERE status IN ('pending', 'sending');
CREATE INDEX email_outbox_ticket_id_idx ON email_outbox (ticket_id);
