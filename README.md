# Rune

> Production-grade Rust autonomous agent framework with a cyberpunk Next.js
> dashboard, Telegram bot, and a fully sandboxed tool layer.

```
        ┌──────────────────────────────────────────────────────────┐
        │                       R   U   N   E                      │
        │           perceive → recall → plan → execute → reflect   │
        └──────────────────────────────────────────────────────────┘
                  │                              │
          ┌───────┴───────┐              ┌───────┴────────┐
          │  Next.js UI   │◀── HTTP/WS──▶│   axum server  │◀── Telegram ─┐
          └───────────────┘              └───────┬────────┘              │
                                                 │                       │
                                  ┌──────────────┼──────────────┐        │
                                  │              │              │        │
                              ┌───┴────┐    ┌────┴───┐    ┌─────┴────┐   │
                              │ SQLite │    │ Qdrant │    │  LLM x7  │   │
                              └────────┘    └────────┘    └──────────┘   │
                                                                         │
                                                              teloxide ──┘
```

Rune is a single-binary Rust agent that loops over LLM planning + tool
execution, persists episodic and semantic memory, and exposes a streaming
REST + WebSocket API. A Next.js dashboard and a Telegram bot are both first-
class clients of the same backend.

---

## Features

- **7 LLM providers** behind a failover router: Gemini, Groq, OpenRouter,
  Fireworks, Anthropic, OpenAI, Ollama.
- **4 sandboxed tools** the agent can call: `terminal` (bash with deny-list +
  timeout), `file` (workspace-canonicalised read/write/list/delete),
  `web_search` (DuckDuckGo), `http_fetch` (allowlisted HTTPS GET).
- **Streaming agent loop** that broadcasts `WsEvent`s (token deltas, tool
  calls, tool results, status, final answer) over a single WebSocket bus.
- **Episodic + semantic memory**: SQLite-backed short term log, Qdrant-
  backed vector recall, reflector that summarises and re-embeds at task end.
- **JWT-authenticated REST API** (`/api/auth/login`, `/api/status`,
  `/api/tasks`, `/api/memory`, `/api/config`, `/api/model`, `/api/agent/*`,
  `/api/ws`).
- **Telegram bot** with `/start /help /status /run /abort /model`, an
  i64 whitelist, and a rate-limited message-edit progress reporter.
- **Cyberpunk Next.js 14 dashboard**: xterm.js live terminal, chat panel,
  tool-call inspector, task history, memory browser, settings.
- **Full docker-compose stack**: backend, dashboard, Qdrant — one command
  to bring it all up.

---

## Quick start

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/SardorchikDev/Rune/main/install.sh | bash
```

The installer runs a preflight (`git`, `docker`, `docker compose`, `openssl`),
clones the repo into `~/rune`, walks you through a config wizard (auto-generated
JWT secret, sha256-hashed dashboard password, one LLM provider key, optional
Telegram bot), writes `backend/.env` + `backend/config.toml` with `0600`
permissions, and brings the full Docker stack up. End-to-end it takes under two
minutes on a clean machine.

Flags worth knowing:

```bash
# Pick the install location and don't auto-start.
curl -fsSL .../install.sh | bash -s -- --dir ~/agents/rune --skip-start

# Update an existing install in place.
curl -fsSL .../install.sh | bash -s -- --update

# Fully scripted (CI / Ansible / etc).
RUNE_DASHBOARD_PASSWORD="…" \
RUNE_DEFAULT_PROVIDER=anthropic \
RUNE_PROVIDER_API_KEY=sk-ant-… \
curl -fsSL .../install.sh | bash -s -- --non-interactive
```

Run `bash install.sh --help` for the full flag list.

### Manual setup (Docker)

```bash
git clone https://github.com/SardorchikDev/Rune.git
cd Rune

cp backend/config.example.toml backend/config.toml
cp backend/.env.example backend/.env

# 1. Generate a JWT secret (>= 32 chars) and a dashboard password hash.
openssl rand -hex 32                            # -> jwt secret
printf '%s' 'choose-a-password' | sha256sum     # -> dashboard hash

# 2. Edit backend/config.toml:
#    - server.jwt_secret                   (or set RUNE_JWT_SECRET in .env)
#    - server.dashboard_password_sha256
#    - llm.providers.<provider>.api_key    (or set RUNE_*_API_KEY in .env)
#    - telegram.bot_token + allowed_user_ids if you want the bot

docker compose up --build
```

Visit `http://localhost:3000/login` once everything is up. Sign in with
the password you hashed. The workspace page connects to the backend on
`ws://localhost:8080/api/ws` and starts streaming as soon as you submit
your first prompt.

---

## Local development

### Prerequisites

- Rust **stable** (pinned via `rust-toolchain.toml`).
- Node.js **22.x**.
- A Qdrant instance (run just the vector DB with `docker compose up qdrant`).

### Backend

```bash
cd backend
cp config.example.toml config.toml
cp .env.example .env
# fill in keys / secrets as described above

cargo run --release
```

| Command | Purpose |
| ------- | ------- |
| `cargo build --release` | Optimised production binary at `target/release/rune`. |
| `cargo test` | Unit + integration tests (no network required). |
| `cargo clippy -- -D warnings` | Lint, treating warnings as errors. |
| `cargo fmt --all -- --check` | Verify formatting. |

### Dashboard

```bash
cd dashboard
npm install
cp .env.local.example .env.local      # point NEXT_PUBLIC_API_URL at backend
npm run dev                           # serves http://localhost:3000
```

| Command | Purpose |
| ------- | ------- |
| `npm run build` | Production build (Next.js standalone). |
| `npm run lint` | Next/ESLint. |
| `npm run typecheck` | `tsc --noEmit`. |

---

## Configuration reference (`backend/config.toml`)

Every field can be overridden via environment variables using the
`RUNE_<SECTION>__<FIELD>` convention (double underscore separates
nesting). A handful of well-known keys also accept friendlier aliases —
see `backend/.env.example`.

### `[server]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `host` | string | Bind address. Default `0.0.0.0`. |
| `port` | u16 | TCP port. Default `8080`. |
| `jwt_secret` | string | **≥ 32 chars**. Used to sign dashboard JWTs. Backend panics on startup if shorter. |
| `cors_origins` | `[string]` | Browser origins allowed via CORS. |
| `dashboard_password_sha256` | string | Hex SHA-256 of the dashboard password. Never plaintext. |

### `[telegram]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `enabled` | bool | Toggle bot startup. |
| `bot_token` | string | Token issued by `@BotFather`. |
| `allowed_user_ids` | `[i64]` | Whitelist. Messages from other users are silently dropped. |

### `[llm]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `default_provider` | string | Provider key (`gemini` / `groq` / …). |
| `default_model` | string | Model name the router uses by default. |
| `stream_tokens` | bool | Stream agent tokens over WebSocket. |
| `max_retries` | u32 | HTTP retry count per provider. |
| `timeout_secs` | u64 | Per-request HTTP timeout. |
| `failover.enabled` | bool | On provider error, try the next provider. |
| `failover.order` | `[string]` | Provider keys in failover order. |
| `providers.<name>.api_key` | string | Provider credential. |
| `providers.<name>.base_url` | string | Override for self-hosted/proxy. |
| `providers.<name>.models` | `[string]` | Suggested models for the dashboard picker. |

### `[memory]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `vector_backend` | string | Currently only `qdrant`. |
| `qdrant_url` | string | Qdrant HTTP endpoint. |
| `collection_name` | string | Qdrant collection for semantic memory. |
| `embedding_provider` | string | `gemini` or `openai`. |
| `embedding_model` | string | Embedding model name. |
| `embedding_dim` | usize | Vector dimensionality. |
| `top_k` | usize | Number of memories injected into context per turn. |

### `[tools]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `workspace_dir` | string | Path the file/terminal tools are confined to. |
| `terminal_timeout_secs` | u64 | Kill bash invocations after this. |
| `allow_web_search` | bool | Toggle the `web_search` tool. |
| `allow_http_fetch` | bool | Toggle the `http_fetch` tool. |
| `http_fetch_allowlist` | `[string]` | Permitted host suffixes for HTTP GET. |

### `[agent]`
| Field | Type | Purpose |
| ----- | ---- | ------- |
| `max_iterations` | u32 | Hard cap on the perceive→reflect loop. |
| `system_prompt_path` | string | Path to the system prompt. |
| `reflection_enabled` | bool | Run the reflector and store summaries in Qdrant. |
| `auto_summarize_threshold` | u32 | Iterations before in-context summarisation kicks in. |

---

## REST + WebSocket API

All `/api/*` routes (except `/api/auth/login`) require
`Authorization: Bearer <jwt>`. The WebSocket reads the same JWT from
`?token=<jwt>` because cookies cannot be set on `ws://` connections.

| Method | Path | Description |
| ------ | ---- | ----------- |
| `POST` | `/api/auth/login` | Trades `{ password }` for `{ token, expires_at }`. Rate-limited via `governor`. |
| `GET`  | `/api/status` | Uptime, active tasks, default provider/model. |
| `POST` | `/api/tasks` | `{ prompt, provider?, model? }` → `{ task_id }`. Spawns the agent loop. |
| `GET`  | `/api/tasks?status=&limit=` | Paged history. |
| `GET`  | `/api/tasks/:id` | Full task plus agent log timeline. |
| `POST` | `/api/agent/abort` | `{ task_id }` → cancels the spawned task handle. |
| `GET`  | `/api/memory?query=&limit=` | Substring search over the SQLite memory index. |
| `DELETE` | `/api/memory/:id` | Drop from index + Qdrant collection. |
| `GET`  | `/api/config` | Full config with every `api_key` masked as `sk-****`. |
| `PUT`  | `/api/config` | Partial config patch. Writes `config.toml` and hot-reloads. |
| `GET`  | `/api/model` | Current `{ provider, model }` plus available providers. |
| `PUT`  | `/api/model` | Hot-swap the default provider/model. |
| `GET`  | `/api/tools` | Tool catalog (JSON Schema definitions). |
| `GET`  | `/api/ws?token=` | WebSocket upgrade. Forwards `WsEvent`s as JSON frames. |

`WsEvent` is a discriminated union (`type` tag) defined in
`backend/src/interfaces/api/ws.rs`. The dashboard mirrors the type in
`dashboard/src/types/ws.ts`.

---

## Adding a new LLM provider

1. Add a file under `backend/src/core/llm/providers/<name>.rs`.
2. Implement the `LlmProvider` trait from `backend/src/core/llm/mod.rs`:
   - `name()`, `supported_models()`, `complete`, `stream`, `embed`.
   - Most OpenAI-compatible APIs can subclass the helper in
     `providers/openai_compatible.rs`.
3. Register the provider in `backend/src/core/llm/router.rs::LlmRouter::new`.
4. Add a `[llm.providers.<name>]` section to `backend/config.example.toml`
   with `api_key`, `base_url`, and a `default_model` or `models` list.
5. The dashboard automatically picks up new providers from `/api/config`.

---

## Adding a new tool

1. Create `backend/src/tools/<name>.rs`.
2. Implement the `Tool` trait from `backend/src/tools/mod.rs`:
   - `name()`, `description()`, `parameters_schema()` (JSON Schema), and
     `execute(params) -> ToolResult`.
3. Register the tool inside `ToolRegistry::new` in `backend/src/tools/mod.rs`.
4. If the tool touches the filesystem, route every path through
   `utils::sanitize::ensure_inside_workspace` to enforce sandboxing.
5. If the tool calls outbound HTTP, honour `tools.http_fetch_allowlist` and
   strip `Authorization` headers before sending.

---

## Security model

- **JWT secret length is enforced at startup.** Less than 32 bytes → panic
  with a clear message.
- **All `/api/*` routes** (except `/api/auth/login`) require a valid JWT.
  401 on signature failure, expiry, or missing header.
- **Dashboard password** is stored as a SHA-256 hex digest.
  `/api/auth/login` SHA-256s the supplied password and compares to the
  stored digest.
- **Rate limiting** on `/api/auth/login` (default 5 attempts / IP / minute)
  via `tower-governor`.
- **CORS** is restricted to `config.server.cors_origins`. The dashboard
  origin must be listed explicitly.
- **Terminal tool** rejects commands matching the deny-list
  (`rm -rf /`, fork bombs, `/dev/sd*`, `shutdown`, `reboot`, `sudo`, `su`)
  before spawning bash. Every execution has a hard timeout from
  `tools.terminal_timeout_secs`.
- **File tool** canonicalises every path and refuses operations whose
  canonical form does not start with the workspace root.
- **HTTP fetch tool** enforces the host allowlist, only allows HTTPS, and
  strips outbound `Authorization` headers.
- **Telegram bot** silently drops messages from any `user_id` not in
  `telegram.allowed_user_ids` and logs the attempt at `WARN`.
- **API key masking**: `/api/config` returns every key as
  `sk-****<last 4>` so an unintended dashboard screenshot cannot leak
  credentials.

---

## Architecture

```
backend/
├── src/
│   ├── main.rs               # bootstraps config, db, router, axum, telegram
│   ├── state.rs              # AppState shared via axum extension
│   ├── config.rs             # config + env override loader
│   ├── error.rs              # AppError / AppResult
│   ├── core/
│   │   ├── db.rs             # sqlx pool + migrations
│   │   ├── jwt.rs            # token mint + verify
│   │   ├── metrics.rs        # usage accounting
│   │   └── llm/              # provider trait + router + 7 providers
│   ├── tools/                # 4 sandboxed tools + ToolRegistry
│   ├── agent/
│   │   ├── loop_.rs          # perceive → recall → plan → execute → reflect
│   │   ├── planner.rs        # prompt assembly + tool extraction
│   │   ├── reflector.rs      # post-run summarisation + memory write
│   │   ├── context.rs        # rolling context window
│   │   └── memory/           # episodic + semantic + embedder
│   ├── interfaces/
│   │   ├── api/              # axum routes + WebSocket + auth middleware
│   │   └── telegram/         # teloxide bot + progress reporter
│   └── utils/                # sanitize, truncate
├── migrations/               # 4 SQLite migrations
├── prompts/system.md         # default agent system prompt
└── Cargo.toml
dashboard/
├── src/
│   ├── app/                  # Next.js 14 app router pages
│   │   ├── login/            # /login
│   │   └── dashboard/        # workspace, tasks, memory, logs, settings
│   ├── components/           # layout + workspace + tasks + memory + settings
│   ├── hooks/                # useWebSocket, useAgent, useTasks, useMemory, useConfig
│   ├── store/                # zustand agentStore + configStore
│   ├── lib/                  # api (ky), auth, ws
│   ├── types/                # api + ws DTOs (mirror Rust types)
│   └── middleware.ts         # JWT cookie gate
├── tailwind.config.ts
└── next.config.js
Dockerfile.backend
Dockerfile.dashboard
docker-compose.yml
```

---

## License

MIT. See `LICENSE` (TBD).
