# CS-ODDS Support

Customer-support ticket system: Rust (Axum + sqlx, hexagonal architecture) backend, Next.js (Redux Toolkit) frontend, Postgres, and S3-compatible storage for attachments.

## Scope of this submission

**Delivered (phase 1):**

| # | Milestone | Status |
|---|---|---|
| 1 | Foundation: Rust API (hexagonal), Next.js 16, Postgres, compose services | ✅ Done |
| 2 | Accounts & roles: register, sign in/out, token refresh with reuse detection, admin/agent/customer roles, team management | ✅ Done |
| 3 | Tickets: customer portal (create, follow, reply, mark solved), agent queue and workspace (filters, search, assign, status/priority, internal notes, history) | ✅ Done |
| 4 | Attachments: files on requests, replies and internal notes; direct-to-storage uploads with progress; permission-checked downloads | ✅ Done |
| 5 | Email notifications: confirmations, replies, solved and assignment emails through an outbox with retries; SMTP (Mailpit locally) | ✅ Done |

**Moved to phase 2** (designed in `docs/ARCHITECTURE.md`, not built):

| # | Milestone | What it adds |
|---|---|---|
| 6 | Email-to-ticket | Inbound email webhook creates tickets and threads replies |
| 7 | SLA | Reply and resolution deadlines, "At risk" view, auto-solve after 7 days waiting on the customer |
| 8 | Dashboard | Volume, response times and agent workload charts |

**Known limitations:** no login rate limiting yet, and "3 minutes ago" timestamps only refresh when the page re-renders.

**Tests:** 77 backend tests (unit, plus full-stack against real Postgres, S3 storage and SMTP). The frontend lints and builds cleanly.

## How to run

### Prerequisites

| Tool | Version | Check |
|---|---|---|
| Rust + Cargo | 1.98 or newer | `cargo --version` |
| Node.js | 20.9 or newer | `node --version` |
| Yarn | 4.x (via `corepack enable`) | `yarn --version` |
| Docker or Podman | with Compose | `docker compose version` / `podman compose version` |

`sqlx-cli` is optional. You only need it to change queries (`cargo install sqlx-cli --no-default-features --features postgres,rustls`).

### 1. Clone and start the services

```sh
git clone https://github.com/W1ch8da/cs-odds.git
cd cs-odds
docker compose up -d        # or: podman compose up -d
```

This starts:

- **Postgres 18** on port 5432.
- **RustFS** on ports 9000 (S3 API) and 9001 (console). RustFS is S3-compatible storage for attachments, used because MinIO no longer publishes community images.
- **Mailpit**, which catches every email the app sends. Browse them at http://localhost:8025.

### 2. Start the backend (terminal 1)

```sh
cd backend
cp .env.example .env
```

Edit `backend/.env` and set two values:

- `JWT_SECRET`: any random string of 32 or more characters, e.g. the output of `openssl rand -hex 32`.
- `ADMIN_PASSWORD`: the password for the first admin account (`ADMIN_EMAIL`, default `admin@cs-odds.local`).

Then run:

```sh
cargo run
```

The `S3_*` values in `.env.example` already match the compose storage service, so you don't need to change them locally.

The API starts on http://localhost:8080. On first start it creates the database tables, the admin account and the storage bucket. You should see `listening on 0.0.0.0:8080`.

### 3. Start the frontend (terminal 2)

```sh
cd frontend
cp .env.example .env.local
yarn install
yarn dev
```

Open http://localhost:3000 and sign in with `ADMIN_EMAIL` / `ADMIN_PASSWORD`.

### 4. (Optional) Load demo data

In a third terminal, with the backend running:

```sh
cd backend && ./scripts/seed_demo.sh
```

This adds agents, customers and sample tickets (see [Demo data](#demo-data)). You can then sign in as an agent (`maya@cs-odds.example`) or a customer (`jordan@northwind.example`). The demo password is at the top of `backend/scripts/seed_demo.sh`.

### Troubleshooting

| Problem | Fix |
|---|---|
| `failed to connect to Postgres` | The database isn't running. Run `docker compose up -d` and wait a few seconds. |
| `JWT_SECRET must be set` / `at least 32 characters` | Set `JWT_SECRET` in `backend/.env` (step 2). |
| `object storage is not reachable` or `S3_ACCESS_KEY must be set` | Storage isn't running, or your `backend/.env` predates attachments. Run `docker compose up -d`, and copy the `S3_*` lines from `.env.example`. |
| An upload shows "couldn't reach file storage" | The browser uploads straight to port 9000. Check that the `s3` service is running and that `S3_PUBLIC_ENDPOINT` is an address your browser can reach. |
| Port 5432, 8080 or 3000 already in use | Stop the other process, or change `BIND_ADDR` (backend) and `BACKEND_URL` (frontend `.env.local`) to match. |
| `yarn: command not found` or wrong Yarn version | Run `corepack enable`, then `yarn install` again. |
| Signed out after restarting with a new `JWT_SECRET` | Expected: old sessions become invalid. Sign in again. |

### Useful URLs

| URL | What |
|---|---|
| http://localhost:3000 | Web app |
| http://localhost:8080/health | API health check |
| http://localhost:8025 | Mailpit: every email the app sends (nothing reaches real inboxes) |
| http://localhost:9001 | RustFS storage console (keys: `S3_ACCESS_KEY` / `S3_SECRET_KEY`) |

## Signing in

- **Admin:** created on first start from `ADMIN_EMAIL` / `ADMIN_PASSWORD` in `backend/.env`. Only created if no admin exists yet.
- **Agents:** an admin adds them under *Team & customers*.
- **Customers:** sign up at `/register`.

Sessions use two httpOnly cookies: a 15-minute access token (JWT) and a 30-day refresh token. Refresh tokens rotate on every use, and replaying an old one signs that user out everywhere.

## Email notifications

Emails are queued in the `email_outbox` table and sent by a background worker every few seconds. A slow or unavailable mail server never slows down or breaks the app. Failed sends are retried after 1 min, 5 min, 15 min, 1 h and 6 h, then marked `failed`.

| Event | Who gets an email |
|---|---|
| Customer submits a request | The customer (confirmation) |
| Agent logs a ticket for a customer | The customer |
| Agent replies (not internal notes) | The customer |
| Agent marks a ticket solved | The customer |
| Customer replies | The assigned agent |
| Ticket assigned to someone else | The new assignee |
| Internal note by someone else | The assigned agent (never the customer) |

Subjects look like `[#TKT-12] …`. Locally everything lands in Mailpit. For real email, set `SMTP_URL` (e.g. `smtps://user:pass@smtp.example.com`) and `MAIL_FROM` in `backend/.env`.

## Demo data

With the API running on a fresh database:

```sh
cd backend && ./scripts/seed_demo.sh
```

This creates two agents (Maya, Daniel), two customers (Jordan, Sofia) and five tickets in different states. The demo password is in the script.

## Checks

```sh
cd backend && cargo test          # needs Postgres, s3 and Mailpit running (integration tests create temp DBs)
SQLX_OFFLINE=true cargo build     # builds without a database using .sqlx/
cd frontend && yarn lint && yarn build
```

## API reference

The REST API is documented in [`openapi.yml`](openapi.yml) (OpenAPI 3.1). It covers every endpoint, request and response body, role rule and error code. To browse it locally:

```sh
yarn dlx -p @redocly/cli redocly build-docs openapi.yml -o api-docs.html   # then open api-docs.html
# or paste openapi.yml into https://editor.swagger.io
```

## Backend layout (Ports & Adapters)

- `src/domain`: entities and business rules, with no I/O.
- `src/application`: use-case services plus their ports (`ports/inbound` traits the app offers, `ports/outbound` traits it needs).
- `src/adapters`: `inbound/http` (Axum), `inbound/jobs` (background email worker), `outbound/postgres` (sqlx), `outbound/s3` (AWS SDK), `outbound/smtp` (lettre) and `outbound/security` (argon2, JWT) implementations.
- `src/main.rs`: the composition root that wires adapters into services.

`domain` and `application` must never import `axum` or `sqlx`.
