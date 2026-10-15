# Fix plan: make the terminal in TS properly edit a `nano` file

## Problem

`nano` opens and works, but **cannot scroll down** when the document it tries to
edit is longer than the terminal viewport.

## Current flow

### Frontend — `src/routes/session/+page.svelte`

- `setupTerminal()` creates the `@xterm/xterm` `Terminal`, loads `FitAddon` and
  `SerializeAddon`, calls `term.open(terminalElement)`, runs `fitAddon.fit()`,
  and observes container size changes with a `ResizeObserver`.
- `setupSsh(key)`:
  - loads the `SessionInfo` for the key and restores prior serialized terminal
    state from `sessionStorage`,
  - subscribes to Tauri events `ssh-output-{key}` and `ssh-error-output-{key}`,
  - attaches `term.onData(...)`, which buffers input and debounces it by 70 ms
    before calling `invoke("send_ssh_input", { input, ip: key })`,
  - attaches mouse-selection copy and the custom key handler,
  - defines `cleanupSsh()` which currently disposes only `onData` and
    `onSelectionChange`.
- There is **no `term.onResize` handler** and **no `resize_ssh_pty` invoke**
  anywhere in the frontend.

### Backend — `src-tauri/src/ssh/`

- `SshInstance::bastion_session` (`ssh_instance.rs`) creates the pty with:

  ```rust
  let (cols, rows) = term_size::dimensions().unwrap_or((220, 50));
  channel.request_pty("xterm-256color", None, Some((cols as u32, rows as u32, 0, 0)))
  ```

  and after the fast-exit probe it calls `request_pty_size(cols, rows, Some(0), Some(0))`
  exactly once with the same host-terminal/fallback size.
- `start_ssh_session` (`start.rs`) stores only `SshInstance { tx, stop_tx }` in
  the registry; the actual `Channel` is moved into the writer thread.
- `SshEngine::spawn_thread_write` (`ssh_engine.rs`) receives `String` payloads
  over an `mpsc` channel and does `channel.write_all(...)` / `channel.flush()`.
- `send_ssh_input` (`input.rs`) sends a plain `String` over that channel.
- `resize.rs` is **empty**, and `lib.rs` registers **no resize command**.

## Root cause

The SSH pty is created with `term_size::dimensions()` — the host console size,
or the hardcoded fallback `(220, 50)` when the GUI app has no console — and is
**never resized to match the actual xterm.js dimensions**.

`nano` (like other curses apps) queries the pty size (`TIOCGWINSZ`) to decide
how big its window is. When the pty reports more rows than xterm.js actually
shows, `nano` believes the cursor is still inside its window when the user
presses the down arrow / PageDown, so it never performs a viewport scroll. The
result is exactly the observed behavior: `nano` works, but long documents appear
"unscrollable".

The fix is to keep the pty size in sync with xterm.js by calling
`Channel::request_pty_size`:

1. once immediately after the SSH session is set up, and
2. on every subsequent xterm resize event.

## Proposed fix

### Backend

1. In `src-tauri/src/ssh/ssh_instance.rs`, introduce a message enum and change
   the `SshInstance.tx` type so the writer thread can receive both input and
   resize requests:

   ```rust
   pub enum ChannelMessage {
       Input(String),
       Resize { cols: u32, rows: u32 },
   }

   pub struct SshInstance {
       pub tx: mpsc::UnboundedSender<ChannelMessage>,
       pub stop_tx: watch::Sender<bool>,
   }
   ```

2. In `src-tauri/src/ssh/ssh_engine.rs`, change `spawn_thread_write` to accept
   `UnboundedReceiver<ChannelMessage>` and handle both variants. `Resize` calls
   `request_pty_size` on the writer thread, which keeps all `Channel` access
   serialized on a single thread:

   ```rust
   match message {
       ChannelMessage::Input(input) => {
           if channel_mut.write_all(input.as_bytes()).is_err() {
               break;
           }
           if channel_mut.flush().is_err() {
               break;
           }
       }
       ChannelMessage::Resize { cols, rows } => {
           if channel_mut.request_pty_size(cols, rows, None, None).is_err() {
               println!("Failed to resize pty to {}x{}", cols, rows);
           }
       }
   }
   ```

3. In `src-tauri/src/ssh/input.rs`, make `send_ssh_input` send
   `ChannelMessage::Input(input)` instead of a raw `String`.

4. In `src-tauri/src/ssh/start.rs` and `src-tauri/src/ssh/reconnect.rs`, create
   the mpsc channel as `mpsc::unbounded_channel::<ChannelMessage>()` so the
   stored `SshInstance` matches the new `tx` type.

5. Implement `src-tauri/src/ssh/resize.rs`:

   ```rust
   use crate::ssh::ssh_engine::SshEngine;
   use crate::ssh::ssh_instance::ChannelMessage;

   #[tauri::command]
   pub fn resize_ssh_pty(
       cols: u32,
       rows: u32,
       ip: String,
       state: tauri::State<'_, SshEngine>,
   ) -> Result<(), String> {
       let registry = state.0.lock().unwrap();
       if let Some(instance) = registry.get(&ip) {
           instance
               .tx
               .send(ChannelMessage::Resize { cols, rows })
               .map_err(|e| e.to_string())?;
           Ok(())
       } else {
           Err("Session not found".into())
       }
   }
   ```

6. Register the command in `src-tauri/src/lib.rs`: add
   `use ssh::resize::resize_ssh_pty;` and add `resize_ssh_pty,` to
   `generate_handler!`.

### Frontend — `src/routes/session/+page.svelte`

1. In `setupSsh(key)`, attach an `onResize` listener that forwards every xterm
   resize to the backend:

   ```ts
   let detachTermOnResize = term.onResize(({ cols, rows }) => {
       invoke("resize_ssh_pty", { cols, rows, ip: key }).catch((e) =>
           console.debug("resize_ssh_pty failed", e),
       );
   });
   ```

2. Immediately after setting up the SSH listeners, sync the initial size once
   (this covers the pty that was created with the fallback size before the
   terminal was open):

   ```ts
   fitAddon.fit();
   const { cols, rows } = term;
   if (cols > 0 && rows > 0) {
       invoke("resize_ssh_pty", { cols, rows, ip: key }).catch((e) =>
           console.debug("resize_ssh_pty failed", e),
       );
   }
   ```

3. In `cleanupSsh()`, dispose the new listener:

   ```ts
   detachTermOnResize.dispose();
   ```

   `fitAddon.fit()` and the existing `ResizeObserver` already trigger
   `term.onResize`, so window resizes and font changes flow through the same
   handler.

### Rejected alternative

Store the `Channel` behind `Arc<Mutex<Channel>>` inside `SshInstance` and call
`request_pty_size` directly from the `resize_ssh_pty` command. Rejected because
it would let the command thread and the writer thread access the libssh2
channel concurrently; the `ChannelMessage` enum keeps all channel operations on
the existing writer thread, matching the current architecture and reducing
libssh2 thread-safety risk.

## Verification

### Static checks (after the fix is implemented)

- `npm run check` — frontend type check (runs `svelte-kit sync` then
  `svelte-check`).
- `cargo check` in `src-tauri/` — verifies the Rust changes compile.

### Manual test for the nano scroll behavior

1. Start the app (`npx tauri dev` or a debug build).
2. Open an SSH session.
3. Inside the terminal, create a file longer than the viewport:

   ```sh
   seq 1 200 > /tmp/long.txt
   nano /tmp/long.txt
   ```

4. Confirm the terminal size matches the pty: `stty size` should report the
   same `rows columns` as the current xterm size.
5. In `nano`, press ↓, PageDown, Ctrl+V, and End repeatedly and confirm the
   viewport scrolls all the way to the bottom of the file.
6. Resize the app window (and change the font size with Ctrl+`+` / Ctrl+`-`)
   and repeat step 4–5; the pty must track the new xterm dimensions.
7. Reconnect to a session and immediately open `nano` before resizing the
   window; the initial pty sync should make scrolling work right away.

### Expected result

`nano` scrolls correctly for documents longer than the terminal viewport, and
the behavior survives window resizes, font changes, and session reconnects.

## Scope note

This document is the fix plan only; it does not modify any source files. The
implementation steps above should be applied and verified in the order listed.

