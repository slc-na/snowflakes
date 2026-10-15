# Implementation Plan

Assumption: keep the existing architecture (one shared `xterm.js` instance per session page, and `TerminalPane` instances in the multiwindow grid) and fix the two regressions with targeted frontend changes — no Rust changes are needed because `resize_ssh_pty` already exists.

## 1. Fix cross-tab terminal bleed in `src/routes/session/+page.svelte`

- Add a session-generation counter (e.g. `let sessionGeneration = 0`) and a `disposeActiveSession()` helper that unlistens `unlisten`/`unlistenError`, calls the current `cleanupSsh()` (disposing `onData`/`onResize`/`onSelection` and clearing the debounce timer), and invalidates in-flight async setup.
- Call `disposeActiveSession()` at the start of the tab-switch branch in the `$effect`, before `term.clear()` and before switching `currentSshKey`/calling `setupSsh`, and also in `onDestroy`.
- Refactor `setupSsh(key, generation)` to use local listener/disposer variables instead of the shared module-level `unlisten`/`cleanupSsh`, and after every `await` (`loadSessionInfo`, `listen`) check that `key === currentSshKey` and the generation is still current; if stale, clean up local listeners and return without writing to the terminal.
- Guard all event callbacks (`ssh-output`, `onData`, `onResize`, `onSelectionChange`) with a `key === currentSshKey` check so a stale tab's output can never write into the currently displayed tab.
- Keep the existing save/load order for `xterm_state_*` (save the old tab before clearing), but only register the new cleanup/disposers when the session is still current.
- Verification: `npm run check`; manual test alternating quickly between a terminal full of output and an empty terminal — each tab must keep only its own content.

## 2. Fix remote pty resize for nano in `src/components/terminal/TerminalPane.svelte`

- Add a `term.onResize(({ cols, rows }) => invoke("resize_ssh_pty", { cols, rows, ip: key }))` handler after session setup, mirroring the behavior already present in `session/+page.svelte`.
- Push the real xterm size once after `fitAddon.fit()` when a session is attached/reconnected (`if (term.cols > 0 && term.rows > 0)`), so the pty starts at the pane's actual split size.
- Harden the resize path: replace the 50ms `setTimeout` in `handleResize` with `requestAnimationFrame(() => fitAddon.fit())`, and ensure the `ResizeObserver` fires when the CSS grid pane changes size (observe the terminal element/pane container directly if the parent observation misses grid layout switches).
- Optionally apply the same generation-guard pattern from phase 1 to `TerminalPane.setupSsh` so reassigning a pane's session cannot leak stale listeners/writes.
- Verification: `npm run check`; manual test in `/window` with a split layout: open `nano`, resize/change the layout, and confirm nano reflows to the pane size (e.g. `$COLUMNS`/`$LINES` match the pane).

## 3. Verification and cleanup

- Run `npm run check` from the repo root.
- Manually test both regressions under `npx tauri dev`: session tab alternation, and multiwindow nano fit across layout changes.
- Review the final diff to confirm only the intended files changed and no debug/scratch code remains.
