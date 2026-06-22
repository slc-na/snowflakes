<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import TerminalPane from "../../components/terminal/TerminalPane.svelte";
    import { loadSessionInfo } from "../../controller/local";
    import type { SessionInfo } from "../../types/settings";

    const WINDOW_STATE_KEY = "snowflakes_window_state";

    let sessions = $state<SessionInfo[]>([]);
    let layout = $state("2x2");

    type PaneState = { id: number; sessionKey: string | null };
    let panes = $state<PaneState[]>(
        Array(6)
            .fill(null)
            .map((_, i) => ({ id: i, sessionKey: null })),
    );

    let isRestoring = true;

    function loadSavedWindowState(): { layout: string; paneKeys: (string | null)[] } | null {
        const raw = localStorage.getItem(WINDOW_STATE_KEY);
        if (!raw) return null;
        try {
            return JSON.parse(raw);
        } catch {
            return null;
        }
    }

    function saveWindowState() {
        if (isRestoring) return;
        localStorage.setItem(
            WINDOW_STATE_KEY,
            JSON.stringify({
                layout,
                paneKeys: panes.map((p) => p.sessionKey),
            }),
        );
    }

    async function loadActiveSessions() {
        const keys = await invoke<string[]>("get_active_session");
        const infos = await Promise.all(keys.map((key) => loadSessionInfo(key)));
        sessions = infos.filter((s): s is SessionInfo => s !== null);
        return new Set(keys);
    }

    onMount(async () => {
        const activeKeys = await loadActiveSessions();

        // Restore the previous layout + pane assignments, but only reattach
        // panes to sessions that are still actually connected on the backend -
        // a closed/dead session shouldn't reappear as a ghost pane.
        const saved = loadSavedWindowState();
        if (saved) {
            if (saved.layout) layout = saved.layout;
            saved.paneKeys.forEach((key, i) => {
                if (panes[i] && key && activeKeys.has(key)) {
                    panes[i].sessionKey = key;
                }
            });
        }

        isRestoring = false;
    });

    const layouts = [
        { id: "1x1", cols: 1, rows: 1, label: "1x1" },
        { id: "2x1", cols: 2, rows: 1, label: "2x1" },
        { id: "1x2", cols: 1, rows: 2, label: "1x2" },
        { id: "2x2", cols: 2, rows: 2, label: "2x2" },
        { id: "3x2", cols: 3, rows: 2, label: "3x2" },
    ];

    let activeLayout = $derived(
        layouts.find((l) => l.id === layout) || layouts[0],
    );
    let activePanes = $derived(
        panes.slice(0, activeLayout.cols * activeLayout.rows),
    );

    function assignSession(paneId: number, e: Event) {
        const select = e.target as HTMLSelectElement;
        panes[paneId].sessionKey = select.value === "" ? null : select.value;
        saveWindowState();
    }

    function clearSession(paneId: number) {
        panes[paneId].sessionKey = null;
        saveWindowState();
    }

    function selectLayout(id: string) {
        layout = id;
        saveWindowState();
    }
</script>

<div class="window-manager">
    <!-- Toolbar -->
    <div class="toolbar">
        <div class="title">MULTI WINDOW</div>
        <div class="layout-selector">
            {#each layouts as l}
                <button
                    class="layout-btn {layout === l.id ? 'active' : ''}"
                    onclick={() => selectLayout(l.id)}
                    title={l.label}
                >
                    {l.label}
                </button>
            {/each}
        </div>
    </div>

    <!-- Grid Container -->
    <div class="grid-container">
        <div
            class="grid-active"
            style="grid-template-columns: repeat({activeLayout.cols}, 1fr); grid-template-rows: repeat({activeLayout.rows}, 1fr);"
        >
            {#each activePanes as pane (pane.id)}
                <div class="pane-cell">
                    {#if pane.sessionKey}
                        <TerminalPane
                            targetKey={pane.sessionKey}
                            onClose={() => clearSession(pane.id)}
                        />
                    {:else}
                        <div class="empty-pane">
                            <div class="empty-pane-content">
                                <svg
                                    width="32"
                                    height="32"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <rect
                                        x="2"
                                        y="3"
                                        width="20"
                                        height="18"
                                        rx="2"
                                        ry="2"
                                    ></rect>
                                    <line x1="2" y1="9" x2="22" y2="9"></line>
                                    <line x1="6" y1="6" x2="6.01" y2="6"></line>
                                </svg>
                                <h3>Select a Session</h3>
                                <div class="select-wrapper">
                                    <select
                                        onchange={(e) =>
                                            assignSession(pane.id, e)}
                                    >
                                        <option value="" disabled selected>
                                            -- Choose Session --
                                        </option>
                                        {#each sessions as s (s.sessionKey)}
                                            <option value={s.sessionKey}>
                                                {s.label || s.targetIp} ({s.username}@{s.targetIp})
                                            </option>
                                        {/each}
                                    </select>
                                </div>
                            </div>
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    </div>
</div>

<style>
    .window-manager {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100%;
        background-color: var(--sf-bg-app);
    }

    .toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 24px;
        background-color: var(--sf-bg-surface);
        border-bottom: 1px solid var(--sf-border);
        flex-shrink: 0;
    }

    .title {
        font-family: var(--sf-font-title, "Space Grotesk");
        font-size: 14px;
        font-weight: 700;
        color: var(--sf-accent);
        letter-spacing: 0.05em;
    }

    .layout-selector {
        display: flex;
        gap: 8px;
    }

    .layout-btn {
        background: transparent;
        border: 1px solid var(--sf-border);
        color: var(--sf-text-secondary);
        font-family: var(--sf-font-ui);
        font-size: 12px;
        padding: 6px 12px;
        border-radius: var(--sf-radius-sm);
        cursor: pointer;
        transition: all 0.2s;
    }

    .layout-btn:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-text-primary);
        border-color: var(--sf-border-hover);
    }

    .layout-btn.active {
        background: rgba(79, 195, 247, 0.1);
        color: var(--sf-accent);
        border-color: var(--sf-accent);
    }

    .grid-container {
        flex: 1;
        padding: 16px;
        overflow: hidden;
    }

    .grid-active {
        display: grid;
        gap: 16px;
        width: 100%;
        height: 100%;
    }

    .pane-cell {
        background: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-md);
        overflow: hidden;
        display: flex;
        flex-direction: column;
    }

    .empty-pane {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(11, 25, 41, 0.4);
    }

    .empty-pane-content {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 16px;
        color: var(--sf-text-secondary);
    }

    .empty-pane-content h3 {
        font-size: 14px;
        font-weight: 500;
        color: var(--sf-text-primary);
        margin: 0;
    }

    .select-wrapper {
        position: relative;
    }

    select {
        appearance: none;
        background: var(--sf-bg-input);
        border: 1px solid var(--sf-border);
        color: var(--sf-text-primary);
        font-family: var(--sf-font-ui);
        font-size: 13px;
        padding: 8px 32px 8px 12px;
        border-radius: var(--sf-radius-md);
        outline: none;
        cursor: pointer;
        min-width: 250px;
        transition: border-color 0.2s;
    }

    select:hover {
        border-color: var(--sf-border-hover);
    }

    select:focus {
        border-color: var(--sf-accent);
    }

    .select-wrapper::after {
        content: "";
        position: absolute;
        right: 12px;
        top: 50%;
        transform: translateY(-50%);
        border-left: 4px solid transparent;
        border-right: 4px solid transparent;
        border-top: 4px solid var(--sf-text-secondary);
        pointer-events: none;
    }
</style>
