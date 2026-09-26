-- Core schema for the ticketing MVP. Attachments and the email outbox are
-- added in their own milestones.

CREATE TYPE user_role AS ENUM ('admin', 'agent', 'customer');
CREATE TYPE ticket_status AS ENUM ('open', 'pending', 'on_hold', 'solved', 'closed');
CREATE TYPE ticket_priority AS ENUM ('low', 'normal', 'high', 'urgent');
CREATE TYPE ticket_channel AS ENUM ('portal', 'email', 'agent');

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL,
    name          TEXT NOT NULL,
    -- NULL for customers auto-created from inbound email who never set a password.
    password_hash TEXT,
    role          user_role NOT NULL DEFAULT 'customer',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_key ON users (lower(email));

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX refresh_tokens_user_id_idx ON refresh_tokens (user_id);

CREATE TABLE sla_policies (
    priority               ticket_priority PRIMARY KEY,
    first_response_minutes INTEGER NOT NULL CHECK (first_response_minutes > 0),
    resolution_minutes     INTEGER NOT NULL CHECK (resolution_minutes > 0)
);
INSERT INTO sla_policies (priority, first_response_minutes, resolution_minutes) VALUES
    ('urgent',  60,    4 * 60),
    ('high',    4 * 60, 24 * 60),
    ('normal',  8 * 60, 3 * 24 * 60),
    ('low',     24 * 60, 7 * 24 * 60);

CREATE TABLE tickets (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    number                BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE, -- shown as TKT-<number>
    subject               TEXT NOT NULL,
    description           TEXT NOT NULL,
    status                ticket_status NOT NULL DEFAULT 'open',
    priority              ticket_priority NOT NULL DEFAULT 'normal',
    channel               ticket_channel NOT NULL,
    requester_id          UUID NOT NULL REFERENCES users (id),
    assignee_id           UUID REFERENCES users (id),
    first_response_due_at TIMESTAMPTZ,
    resolution_due_at     TIMESTAMPTZ,
    first_responded_at    TIMESTAMPTZ,
    resolved_at           TIMESTAMPTZ,
    sla_breached          BOOLEAN NOT NULL DEFAULT false,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX tickets_requester_id_idx ON tickets (requester_id);
CREATE INDEX tickets_assignee_id_idx ON tickets (assignee_id);
CREATE INDEX tickets_status_priority_idx ON tickets (status, priority);

CREATE TABLE ticket_comments (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id        UUID NOT NULL REFERENCES tickets (id) ON DELETE CASCADE,
    author_id        UUID NOT NULL REFERENCES users (id),
    body             TEXT NOT NULL,
    is_internal      BOOLEAN NOT NULL DEFAULT false,
    email_message_id TEXT UNIQUE, -- Message-ID for inbound/outbound email threading
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ticket_comments_ticket_id_idx ON ticket_comments (ticket_id, created_at);

CREATE TABLE ticket_events (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id  UUID NOT NULL REFERENCES tickets (id) ON DELETE CASCADE,
    actor_id   UUID REFERENCES users (id), -- NULL for system events (e.g. SLA breach)
    kind       TEXT NOT NULL,              -- status_changed, assigned, priority_changed, ...
    old_value  TEXT,
    new_value  TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ticket_events_ticket_id_idx ON ticket_events (ticket_id, created_at);
