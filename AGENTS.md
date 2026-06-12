# Snowflakes — AGENTS.md

## Stack

- **Tauri v2** desktop app (macOS/Windows/Linux)
- **SvelteKit 5** + **adapter-static** (SSG, no SSR) — `export const ssr = false`
- **Tailwind CSS v4** via `@tailwindcss/vite`
- **Rust** backend with libssh2 (`ssh2` crate), SFTP, OAuth2
- **xterm.js** for SSH terminal UI

## Dev commands

| Command | What it does |
|---|---|
| `npm run dev` | Vite frontend dev server (port 1420, strict) |
| `npm run build` | Static build to `build/` |
| `npm run check` | **Always run this** — runs `svelte-kit sync` then `svelte-check` |
| `npm run tauri` | Tauri CLI passthrough |
| `npx tauri dev` | Full Tauri dev mode (Rust + frontend) |
| `cargo build` | Rust backend only |

When in doubt: `npm run check` (no separate `tsc` or `svelte-check` direct call).

## Architecture

```
src/               SvelteKit frontend
├── controller/    Invoke wrappers for Tauri commands (ssh.ts, sftp.ts, local.ts, vault.ts, session.ts)
├── routes/        SvelteKit pages (/, /login, /session, /files, /hosts, /ports, /window, /recents, /settings)
├── components/    Reusable Svelte components (TitleBar, Sidebar, modals, terminal, file browser)
├── types/         TypeScript type definitions
├── constants/     Config constants
└── layout.css     Tailwind import + CSS custom properties (dark/light themes)

src-tauri/         Rust/Tauri backend
├── src/
│   ├── lib.rs     Plugin registration, Tauri command handler registration, app setup
│   ├── main.rs    Windows subsystem attr + calls snowflakes_lib::run()
│   ├── ssh/       SSH session management (start, disconnect, reconnect, input, resize)
│   ├── sftp/      SFTP operations (list dir, download, upload)
│   ├── oauth/     OAuth2 flow (auth server, token management via OS keyring)
│   ├── http/      REST client to backend API (server list, shell config)
│   ├── store/     User info persistence (Stronghold)
│   └── config/    Environment variable loader (dotenvy_macro)
└── tauri.conf.json  Window config, plugins, app identifier, build steps
```

## Critical gotchas

### Env vars are compile-time Rust macros
`.env` must live **inside `src-tauri/`**, not the project root. Values are baked into the Rust binary at compile time via `dotenvy_macro::dotenv!()`. Changing an env var requires a full Rust recompilation (`cargo build` / `npx tauri dev`).

Required vars: `CLIENT_ID`, `REDIRECT_URI`, `SCOPE`, `RESPONSE_TYPE`, `STATE`, `SECRET_KEY`, `BASE_URI`, `BACKEND_URL`, `BASTION_IP`, `PORT_TEMP_SERVER`.

### Custom window chrome
`tauri.conf.json` sets `decorations: false` and `transparent: true`. The title bar is a custom Svelte component (`TitleBar.svelte`) with `data-tauri-drag-region`. The app has a 32px top padding to account for this.

### SSR is disabled
All routes are statically pre-rendered. `+layout.ts` sets `export const prerender = true` and `export const ssr = false`. Don't add server load functions or `+page.server.ts` files expecting SSR.

### Credential storage
- OAuth tokens → OS **keyring** (`keyring` crate, service name `snow-flakes`)
- SSH passwords → **Tauri Stronghold** vault (`src/controller/vault.ts`)
- Session metadata → **localStorage** (prefix `session:`)
- Terminal state → **sessionStorage** (prefix `xterm_state_`)
- Settings → **localStorage** (key `snowflakes_settings`)

### Session communication
SSH output streams from Rust to the frontend via Tauri events (`ssh-output-{key}`, `ssh-error-output-{key}`). The session page listens per-key. SSH input is debounced (70ms) and sent via `invoke("send_ssh_input", ...)`.

### No tests
No test runner or test files exist. `build/` and `.svelte-kit/` are gitignored.

### Rust lib crate naming
The Rust library crate is named `snowflakes_lib` (not `snowflakes`) to avoid a naming collision on Windows (see Cargo.toml comment).

### Routes
OAuth guard in `+layout.ts` redirects unauthenticated users to `/login`. The `/session` page expects a `?key=` query param. All other routes are standalone pages inside their own route directory with a `+page.svelte`.
