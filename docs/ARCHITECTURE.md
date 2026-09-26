# CS-ODDS Architecture & Project Structure

Target layout for the full MVP (milestones 1–8). Items marked ✅ exist today. Everything else is planned.

```
CS-ODDS/
├── compose.yml                  ✅ Postgres 18, MinIO (S3), Mailpit (SMTP)
├── README.md                    ✅
├── docs/ARCHITECTURE.md         ✅ this file
├── backend/                     Rust · Axum · sqlx · Ports & Adapters
└── frontend/                    Next.js 16 · Redux Toolkit · shadcn/ui
```

---

## Backend: Ports & Adapters (single crate)

Dependencies point inward: `adapters → application → domain`. The `domain` and `application` layers never import `axum`, `sqlx`, `lettre` or the S3 SDK.

```
backend/
├── Cargo.toml                   ✅
├── .env.example                 ✅
├── .sqlx/                       offline query metadata (cargo sqlx prepare), committed
├── migrations/
│   ├── 20260926000001_init.sql  ✅ users, refresh_tokens, sla_policies, tickets, comments, events
│   ├── …_attachments.sql        M4
│   └── …_email_outbox.sql       M5
├── src/
│   ├── main.rs                  ✅ entry: config → wiring → serve
│   ├── lib.rs                   ✅
│   ├── config.rs                ✅ env config (DB, JWT secret, SMTP, S3, cookie flags)
│   ├── wiring.rs                ✅ composition root, shared by main.rs and integration tests
│   │
│   ├── domain/                  pure model, no I/O
│   │   ├── error.rs             DomainError (validation / invariant violations)
│   │   ├── user.rs              User, UserId, Role, Email (validated)  ✅
│   │   ├── ticket.rs            Ticket, TicketNumber, Status (+ allowed transitions),
│   │   │                        Priority, Channel  ✅
│   │   ├── comment.rs           Comment, Visibility (public / internal)  ✅
│   │   ├── attachment.rs        Attachment, size/type limits                   M4
│   │   ├── sla.rs               SlaPolicy, due-date calculation, breach rules  M7
│   │   └── ticket_event.rs      audit events  ✅
│   │
│   ├── application/
│   │   ├── error.rs             ✅ AppError (no HTTP/DB types)
│   │   ├── principal.rs         Principal {user_id, role} + authorization helpers  ✅
│   │   ├── ports/
│   │   │   ├── inbound/         what the app OFFERS (use-case traits)
│   │   │   │   ├── health.rs            ✅
│   │   │   │   ├── auth.rs              register, login, refresh, logout, authenticate  ✅
│   │   │   │   ├── users.rs             create staff, list users/agents  ✅
│   │   │   │   ├── tickets.rs           create, list/filter, get, update, assign  ✅
│   │   │   │   ├── comments.rs          reply, internal note, timeline  ✅
│   │   │   │   ├── attachments.rs       presign upload/download, confirm                M4
│   │   │   │   ├── inbound_email.rs     ingest parsed email → ticket/comment            M6
│   │   │   │   ├── sla.rs               policies CRUD, breach sweep                     M7
│   │   │   │   └── dashboard.rs         summary metrics                                 M8
│   │   │   └── outbound/        what the app NEEDS (driven-side traits)
│   │   │       ├── database_probe.rs    ✅
│   │   │       ├── user_repository.rs, refresh_token_repository.rs  ✅
│   │   │       ├── password_hasher.rs, access_token_codec.rs,
│   │   │       │   token_generator.rs, clock.rs  ✅
│   │   │       ├── ticket_repository.rs, comment_repository.rs, event_repository.rs  ✅
│   │   │       ├── file_storage.rs, attachment_repository.rs                           M4
│   │   │       ├── mail_outbox.rs, mail_sender.rs                                      M5
│   │   │       ├── sla_policy_repository.rs                                            M7
│   │   │       └── dashboard_query.rs   (read model)                                   M8
│   │   └── services/            use-case implementations (one per inbound port)
│   │       ├── health.rs ✅  auth.rs  users.rs  tickets.rs  comments.rs
│   │       └── attachments.rs  notifications.rs  inbound_email.rs  sla.rs  dashboard.rs
│   │
│   └── adapters/
│       ├── inbound/             DRIVING adapters (things that call the app)
│       │   ├── http/            ✅ Axum
│       │   │   ├── mod.rs       ✅ router, CORS, tracing
│       │   │   ├── state.rs     ✅ Arc<dyn InboundPort> per use case
│       │   │   ├── error.rs     ✅ AppError → HTTP JSON
│       │   │   ├── extractors.rs   CurrentUser (cookie or Bearer)  ✅
│       │   │   ├── cookies.rs      auth cookie builders  ✅
│       │   │   ├── dto/            request/response types per resource
│       │   │   └── handlers/       health ✅, auth, users, tickets, comments,
│       │   │                       attachments, sla, dashboard, webhooks
│       │   └── jobs/            timer-driven adapters
│       │       ├── outbox_worker.rs     sends queued emails                    M5
│       │       └── sla_worker.rs        marks breaches every minute            M7
│       └── outbound/            DRIVEN adapters (things the app calls)
│           ├── postgres/        ✅ sqlx: repositories, DB enum types, error mapping
│           ├── security/        argon2 hasher, JWT codec, random tokens  ✅
│           ├── system/          clock  ✅
│           ├── s3/              file storage + presigned URLs                  M4
│           └── smtp/            lettre mail sender                             M5
└── tests/                       full-stack tests (real Postgres via #[sqlx::test])
    ├── auth_flow.rs  tickets_flow.rs  inbound_email.rs
```

**Adding a feature follows these steps:**
1. Model it in `domain/`.
2. Declare an inbound port for what the app offers and outbound ports for what it needs.
3. Implement the service.
4. Implement the outbound adapters.
5. Add HTTP handlers and DTOs.
6. Wire it up in `wiring.rs`.

---

## Frontend: Next.js 16 + Redux Toolkit, feature-based

```
frontend/src/
├── proxy.ts                     ✅ route guard (Next 16's replacement for middleware)
├── app/                         routes only: thin pages that compose feature components
│   ├── layout.tsx               ✅ fonts, StoreProvider, Toaster
│   ├── page.tsx                 ✅ → will redirect to the role's home
│   ├── (auth)/                  centred-card layout, no app chrome
│   │   ├── login/page.tsx  ✅
│   │   └── register/page.tsx  ✅
│   ├── portal/                  CUSTOMER: simple top-nav layout
│   │   ├── page.tsx             my tickets  ✅
│   │   ├── new/page.tsx         submit a request  ✅
│   │   └── tickets/[number]/page.tsx   conversation view  ✅
│   ├── agent/                   AGENT/ADMIN: sidebar workspace layout
│   │   ├── tickets/page.tsx     queue (views: Mine · Unassigned · All · At risk)  ✅
│   │   ├── tickets/new/page.tsx create on behalf of a customer  ✅
│   │   ├── tickets/[number]/page.tsx   ticket workspace (thread + side panel)  ✅
│   │   └── dashboard/page.tsx   metrics                                          M8
│   └── admin/                   ADMIN
│       ├── users/page.tsx       team & customers  ✅
│       └── sla/page.tsx         SLA policies                                     M7
│
├── features/                    one folder per domain feature
│   ├── health/                  ✅ healthApi.ts, SystemStatus.tsx
│   ├── auth/                    authApi, authSlice (current user), LoginForm, RegisterForm,
│   │                            UserMenu, RequireRole  ✅
│   ├── users/                   usersApi, UserTable, CreateStaffDialog  ✅
│   ├── tickets/                 ticketsApi, ticketsSlice (queue filters/view), TicketTable,
│   │                            TicketFilters, TicketForm, StatusBadge, PriorityBadge,
│   │                            AssigneeSelect  ✅
│   ├── portal/                  ✅ PortalHome, NewRequestForm, RequestConversation (customer side)
│   ├── attachments/             attachmentsApi, FileDropzone, AttachmentList      M4
│   ├── sla/                     slaApi, SlaIndicator ("due in 2h"), SlaPolicyForm M7
│   └── dashboard/               dashboardApi, KpiTiles, charts                    M8
│
├── components/
│   ├── ui/                      ✅ shadcn primitives
│   └── layout/                  AppShell, Sidebar, TopNav, PageHeader, EmptyState
└── lib/
    ├── store/                   ✅ store.ts, api.ts (baseApi, re-auth on 401), hooks.ts, StoreProvider.tsx
    ├── format.ts                dates, relative time, "TKT-123"
    └── utils.ts                 ✅ cn()
```

**Frontend conventions:**
- `app/` pages stay thin and compose components from `features/`.
- Server data goes through RTK Query; each feature adds its endpoints with `injectEndpoints`.
- Redux slices hold UI and client state only (current user, queue filters), never copies of server data.
- URLs use the human-readable ticket number (`/agent/tickets/123`), not the UUID.

## Roles → landing pages

| Role | Lands on | Can reach |
|---|---|---|
| Customer | `/portal` | own tickets only, public replies only |
| Agent | `/agent/tickets` | all tickets, internal notes, dashboard |
| Admin | `/agent/tickets` | everything an agent can reach, plus `/admin/*` |
