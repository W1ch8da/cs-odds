-- Files uploaded straight to object storage. A row starts as a draft
-- (ticket_id NULL) when the upload slot is issued, and is linked to a ticket
-- (and optionally a comment) when the ticket or reply is sent.
CREATE TABLE attachments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    uploader_id  UUID NOT NULL REFERENCES users (id),
    ticket_id    UUID REFERENCES tickets (id) ON DELETE CASCADE,
    -- NULL on a linked attachment means it belongs to the ticket description.
    comment_id   UUID REFERENCES ticket_comments (id) ON DELETE CASCADE,
    storage_key  TEXT NOT NULL UNIQUE,
    filename     TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL CHECK (size_bytes > 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    linked_at    TIMESTAMPTZ,
    CHECK (comment_id IS NULL OR ticket_id IS NOT NULL),
    CHECK ((ticket_id IS NULL) = (linked_at IS NULL))
);

CREATE INDEX attachments_ticket_id_idx ON attachments (ticket_id);
-- Finds abandoned drafts for cleanup.
CREATE INDEX attachments_drafts_idx ON attachments (created_at) WHERE ticket_id IS NULL;
