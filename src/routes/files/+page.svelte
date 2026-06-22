<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/stores";

    import type { SessionInfo } from "../../types/settings";
    import { loadAllSessions, loadSessionInfo, deleteSessionInfo } from "../../controller/local";
    import { reconnectToSession } from "../../controller/ssh";
    import { deleteSessionPass } from "../../controller/vault";
    import { clearTerminalState } from "../../controller/session";

    import * as sftp from "../../controller/sftp";

    import FileSystem from "../../components/file/FileSystem.svelte";
    import NewHostModal from "../../components/modal/NewHostModal.svelte";
    import { User } from "@lucide/svelte";

    // ─── Page State ───────────────────────────────────────────────────────────
    let isModalOpen = $state(false);
    let sessions = $state<SessionInfo[]>([]);
    let prefillSession = $state<SessionInfo | null>(null);
    let isLoading = $state(true);

    let isConnecting = $state(false);
    let connectingStatus = $state("");

    // ─── SFTP States ──────────────────────────────────────────────────────────
    let sftpSession = $state<{ key: string; info: SessionInfo } | null>(null);
    let isSftpConnecting = $state(false);

    // ─── Toast System ─────────────────────────────────────────────────────────
    type ToastType = "error" | "success" | "info";
    interface Toast {
        id: number;
        message: string;
        type: ToastType;
    }

    let toasts = $state<Toast[]>([]);
    let toastCounter = 0;

    function showToast(message: string, type: ToastType = "error") {
        const id = ++toastCounter;
        toasts = [...toasts, { id, message, type }];
        setTimeout(() => {
            toasts = toasts.filter((t) => t.id !== id);
        }, 4500);
    }

    function dismissToast(id: number) {
        toasts = toasts.filter((t) => t.id !== id);
    }

    // ─── Session Info Modal (User button) ─────────────────────────────────────
    let userModalOpen = $state(false);
    let userModalSession = $state<SessionInfo | null>(null);
    let userModalTab = $state<"info" | "sftp">("info");

    // Custom SFTP login (Tab 2)
    let customSftpUsername = $state("");
    let customSftpPassword = $state("");
    let isCustomSftpConnecting = $state(false);

    function openUserModal(e: MouseEvent, session: SessionInfo) {
        e.stopPropagation();
        userModalSession = session;
        userModalTab = "info";
        customSftpUsername = "";
        customSftpPassword = "";
        userModalOpen = true;
    }

    function closeUserModal() {
        userModalOpen = false;
        userModalSession = null;
    }

    async function handleCustomSftpConnect() {
        if (!userModalSession) return;
        if (!customSftpUsername.trim()) {
            showToast("Username is required.", "error");
            return;
        }

        isCustomSftpConnecting = true;
        try {
            // Build a modified session with the custom username (and optional password)
            const customSession: SessionInfo = {
                ...userModalSession,
                username: customSftpUsername.trim(),
            };

            const key = await sftp.connectToSftpSession(
                customSession,
                (status) => {
                    console.log("[Custom SFTP Status]:", status);
                },
            );

            sftpSession = { key, info: customSession };
            closeUserModal();
            showToast("SFTP connected successfully.", "success");
        } catch (err) {
            showToast(
                err instanceof Error ? err.message : String(err),
                "error",
            );
        } finally {
            isCustomSftpConnecting = false;
        }
    }

    // ─── Lifecycle ────────────────────────────────────────────────────────────
    onMount(async () => {
        try {
            sessions = await loadAllSessions();

            const key = $page.url.searchParams.get("key");
            if (key) {
                const info = (await loadSessionInfo(key)) ?? {
                    sessionKey: key,
                    username: "",
                    targetIp: key,
                    bastionIp: "",
                    label: key,
                    connectedAt: Date.now(),
                };
                sftpSession = { key, info };
            }
        } catch (err) {
            console.error("[Home] Failed to load sessions:", err);
            showToast("Failed to load sessions.", "error");
        } finally {
            isLoading = false;
        }
    });

    // ─── Handlers ─────────────────────────────────────────────────────────────
    async function handleCardClick(session: SessionInfo) {
        isConnecting = true;
        connectingStatus = "Connecting…";
        try {
            const key = await reconnectToSession(session, (msg) => {
                connectingStatus = msg;
            });
            goto(`/session?key=${key}`, {
                state: {
                    bastion: session.bastionIp,
                    initialUsername: session.username,
                    hostname: session.targetIp,
                },
            });
        } catch (e) {
            showToast(String(e), "error");
            isConnecting = false;
        }
    }

    async function handleSftpClick(e: MouseEvent, session: SessionInfo) {
        e.stopPropagation();
        isSftpConnecting = true;

        try {
            const key = await sftp.connectToSftpSession(session, (status) => {
                console.log("[SFTP Status]:", status);
            });
            sftpSession = { key, info: session };
            showToast("SFTP connected.", "success");
        } catch (err) {
            showToast(
                err instanceof Error ? err.message : String(err),
                "error",
            );
        } finally {
            isSftpConnecting = false;
        }
    }

    async function handleDeleteClick(e: MouseEvent, sessionKey: string) {
        e.stopPropagation();
        if (confirm("Are you sure you want to delete this host?")) {
            try {
                await deleteSessionInfo(sessionKey);
                await deleteSessionPass(sessionKey);
                clearTerminalState(sessionKey);
                sessions = sessions.filter((s) => s.sessionKey !== sessionKey);
                showToast("Host removed.", "info");
            } catch (err) {
                console.error("[Home] Failed to delete session:", err);
                showToast("Failed to delete host.", "error");
            }
        }
    }

    function handleNewHost() {
        prefillSession = null;
        isModalOpen = true;
    }

    function formatDate(ts: number): string {
        return new Date(ts).toLocaleString();
    }

    async function closeSftp() {
        if (sftpSession) {
            try {
                await sftp.disconnectSftp(sftpSession.key);
            } catch (e) {
                console.warn("[SFTP] Disconnect failed during close:", e);
            }
        }
        sftpSession = null;
    }
</script>

<!-- ─── Toast Container ──────────────────────────────────────────────────── -->
<div class="toast-container" aria-live="polite">
    {#each toasts as toast (toast.id)}
        <div class="toast toast-{toast.type}" role="alert">
            <div class="toast-snowflake" aria-hidden="true">
                {#if toast.type === "error"}
                    <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <!-- Snowflake SVG -->
                        <line x1="12" y1="2" x2="12" y2="22" />
                        <line x1="2" y1="12" x2="22" y2="12" />
                        <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
                        <line x1="19.07" y1="4.93" x2="4.93" y2="19.07" />
                        <circle cx="12" cy="12" r="2" />
                        <circle cx="12" cy="2" r="1.2" />
                        <circle cx="12" cy="22" r="1.2" />
                        <circle cx="2" cy="12" r="1.2" />
                        <circle cx="22" cy="12" r="1.2" />
                    </svg>
                {:else if toast.type === "success"}
                    <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="20 6 9 17 4 12" />
                    </svg>
                {:else}
                    <svg
                        width="18"
                        height="18"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <circle cx="12" cy="12" r="10" />
                        <line x1="12" y1="8" x2="12" y2="12" />
                        <line x1="12" y1="16" x2="12.01" y2="16" />
                    </svg>
                {/if}
            </div>
            <span class="toast-msg">{toast.message}</span>
            <button
                class="toast-close"
                onclick={() => dismissToast(toast.id)}
                aria-label="Dismiss"
            >
                <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                >
                    <line x1="18" y1="6" x2="6" y2="18" />
                    <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
            </button>
            <div class="toast-progress"></div>
        </div>
    {/each}
</div>

<!-- ─── Main Layout ──────────────────────────────────────────────────────── -->
<main>
    <div class="app-shell">
        <div class="content-area">
            <div class="table-wrapper">
                <div class="table-container">
                    <table class="session-table">
                        <thead>
                            <tr>
                                <th style="width:30%">Host / Alias</th>
                                <th style="width:30%">Address</th>
                                <th style="width:22%">Last Access</th>
                                <th style="width:18%; text-align:right"
                                    >Actions</th
                                >
                            </tr>
                        </thead>
                        <tbody>
                            {#if isLoading}
                                {#each Array(3) as _}
                                    <tr class="skeleton-row">
                                        <td colspan="4">
                                            <div class="shimmer"></div>
                                        </td>
                                    </tr>
                                {/each}
                            {:else if sessions.length === 0}
                                <tr>
                                    <td colspan="4" class="empty-state">
                                        <div class="empty-inner">
                                            <svg
                                                width="36"
                                                height="36"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                                opacity="0.3"
                                            >
                                                <rect
                                                    x="2"
                                                    y="3"
                                                    width="20"
                                                    height="14"
                                                    rx="2"
                                                />
                                                <line
                                                    x1="8"
                                                    y1="21"
                                                    x2="16"
                                                    y2="21"
                                                />
                                                <line
                                                    x1="12"
                                                    y1="17"
                                                    x2="12"
                                                    y2="21"
                                                />
                                            </svg>
                                            <span>No saved sessions</span>
                                        </div>
                                    </td>
                                </tr>
                            {:else}
                                {#each sessions as session (session.sessionKey)}
                                    <tr class="session-row">
                                        <td class="td-info">
                                            <div class="host-main">
                                                <div class="host-dot"></div>
                                                <span class="host-label">
                                                    {session.label ||
                                                        "Untitled Server"}
                                                </span>
                                            </div>
                                        </td>
                                        <td class="td-address">
                                            <code
                                                >{session.username}@{session.targetIp}</code
                                            >
                                        </td>
                                        <td class="td-time">
                                            {formatDate(session.connectedAt)}
                                        </td>
                                        <td class="td-actions">
                                            <div class="action-group">
                                                <!-- User / Session Info -->
                                                <button
                                                    class="icon-btn user"
                                                    onclick={(e) =>
                                                        openUserModal(
                                                            e,
                                                            session,
                                                        )}
                                                    title="Session Info / Custom SFTP Login"
                                                >
                                                    <User size={15} />
                                                </button>
                                                <!-- SFTP -->
                                                <button
                                                    class="icon-btn sftp"
                                                    onclick={(e) =>
                                                        handleSftpClick(
                                                            e,
                                                            session,
                                                        )}
                                                    title="Open SFTP Browser"
                                                >
                                                    <svg
                                                        width="15"
                                                        height="15"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                    >
                                                        <path
                                                            d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                                                        />
                                                    </svg>
                                                </button>
                                                <!-- Delete -->
                                                <button
                                                    class="icon-btn delete"
                                                    onclick={(e) =>
                                                        handleDeleteClick(
                                                            e,
                                                            session.sessionKey,
                                                        )}
                                                    title="Remove"
                                                >
                                                    <svg
                                                        width="15"
                                                        height="15"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                    >
                                                        <path
                                                            d="M3 6h18m-2 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                                        />
                                                    </svg>
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                {/each}
                            {/if}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>

    <!-- ─── Loading Overlay ──────────────────────────────────────────────── -->
    {#if isSftpConnecting || isConnecting}
        <div class="glass-overlay">
            <div class="loader-box">
                <div class="spinner"></div>
                <p class="loader-status">
                    {isSftpConnecting
                        ? "Establishing SFTP Tunnel…"
                        : connectingStatus}
                </p>
            </div>
        </div>
    {/if}

    <!-- ─── SFTP File System ──────────────────────────────────────────────── -->
    {#if sftpSession}
        <FileSystem
            sessionKey={sftpSession.key}
            sessionInfo={sftpSession.info}
            onClose={closeSftp}
        />
    {/if}

    <!-- ─── New Host Modal ────────────────────────────────────────────────── -->
    <NewHostModal
        isOpen={isModalOpen}
        onClose={() => (isModalOpen = false)}
        prefill={prefillSession}
    />
</main>

<!-- ─── User / Session Info Modal ────────────────────────────────────────── -->
{#if userModalOpen && userModalSession}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-backdrop" onclick={closeUserModal}>
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="modal-card" onclick={(e) => e.stopPropagation()}>
            <!-- Header -->
            <div class="modal-header">
                <div class="modal-title-row">
                    <div class="modal-icon">
                        <User size={18} />
                    </div>
                    <div>
                        <h2 class="modal-title">Session Info</h2>
                        <p class="modal-subtitle">
                            {userModalSession.label || "Untitled Server"}
                        </p>
                    </div>
                </div>
                <button
                    class="modal-close-btn"
                    onclick={closeUserModal}
                    aria-label="Close"
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                    >
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                </button>
            </div>

            <!-- Tabs -->
            <div class="modal-tabs">
                <button
                    class="modal-tab {userModalTab === 'info' ? 'active' : ''}"
                    onclick={() => (userModalTab = "info")}
                >
                    Session Details
                </button>
                <button
                    class="modal-tab {userModalTab === 'sftp' ? 'active' : ''}"
                    onclick={() => (userModalTab = "sftp")}
                >
                    Custom SFTP Login
                </button>
            </div>

            <!-- Tab: Session Info (read-only) -->
            {#if userModalTab === "info"}
                <div class="modal-body">
                    <div class="info-grid">
                        <div class="info-row">
                            <span class="info-label">Label</span>
                            <span class="info-value"
                                >{userModalSession.label || "—"}</span
                            >
                        </div>
                        <div class="info-row">
                            <span class="info-label">Username</span>
                            <code class="info-code"
                                >{userModalSession.username}</code
                            >
                        </div>
                        <div class="info-row">
                            <span class="info-label">Target IP</span>
                            <code class="info-code"
                                >{userModalSession.targetIp}</code
                            >
                        </div>
                        {#if userModalSession.bastionIp}
                            <div class="info-row">
                                <span class="info-label">Bastion IP</span>
                                <code class="info-code"
                                    >{userModalSession.bastionIp}</code
                                >
                            </div>
                        {/if}
                        <div class="info-row">
                            <span class="info-label">Session Key</span>
                            <code class="info-code mono-small"
                                >{userModalSession.sessionKey}</code
                            >
                        </div>
                        <div class="info-row">
                            <span class="info-label">Last Connected</span>
                            <span class="info-value"
                                >{formatDate(
                                    userModalSession.connectedAt,
                                )}</span
                            >
                        </div>
                    </div>
                    <p class="info-readonly-note">
                        <svg
                            width="12"
                            height="12"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            ><rect
                                x="3"
                                y="11"
                                width="18"
                                height="11"
                                rx="2"
                            /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg
                        >
                        Read-only — edit via <strong>New Host</strong>
                    </p>
                </div>

                <!-- Tab: Custom SFTP Login -->
            {:else}
                <div class="modal-body">
                    <p class="sftp-tab-desc">
                        Connect to <code class="info-code"
                            >{userModalSession.targetIp}</code
                        > via SFTP using a different user account.
                    </p>
                    <div class="form-group">
                        <label class="form-label" for="sftp-user"
                            >Username</label
                        >
                        <input
                            id="sftp-user"
                            class="form-input"
                            type="text"
                            placeholder="e.g. deploy"
                            bind:value={customSftpUsername}
                            onkeydown={(e) =>
                                e.key === "Enter" && handleCustomSftpConnect()}
                        />
                    </div>
                    <div class="form-group">
                        <label class="form-label" for="sftp-pass"
                            >Password <span class="optional">(optional)</span
                            ></label
                        >
                        <input
                            id="sftp-pass"
                            class="form-input"
                            type="password"
                            placeholder="Leave blank for key-based auth"
                            bind:value={customSftpPassword}
                            onkeydown={(e) =>
                                e.key === "Enter" && handleCustomSftpConnect()}
                        />
                    </div>
                    <button
                        class="btn-connect"
                        onclick={handleCustomSftpConnect}
                        disabled={isCustomSftpConnecting}
                    >
                        {#if isCustomSftpConnecting}
                            <span class="btn-spinner"></span>
                            Connecting…
                        {:else}
                            <svg
                                width="15"
                                height="15"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                            >
                                <path
                                    d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                                />
                            </svg>
                            Connect via SFTP
                        {/if}
                    </button>
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    /* Scopes the SFTP overlay below to this page's content area instead of
       the whole viewport, so it doesn't paint over the sidebar/tab bar. */
    main {
        position: relative;
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    /* ── App Shell ───────────────────────────────────────────────────────── */
    .app-shell {
        width: 100%;
        height: 100%;
        display: flex;
        flex-direction: column;
        background: var(--sf-bg-app);
        color: var(--sf-text-primary);
        font-family: "Inter", "Segoe UI", system-ui, sans-serif;
    }

    .content-area {
        flex: 1;
        padding: 28px 32px;
        overflow-y: auto;
    }

    .table-wrapper {
        max-width: 1100px;
        margin: 0 auto;
    }

    /* ── Table ───────────────────────────────────────────────────────────── */
    .table-container {
        background: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: var(--radius-lg);
        overflow: hidden;
    }

    .session-table {
        width: 100%;
        border-collapse: collapse;
        font-size: 13px;
        table-layout: fixed;
    }

    .session-table th {
        text-align: left;
        padding: 13px 20px;
        color: var(--sf-text-muted);
        font-weight: 600;
        border-bottom: 1px solid var(--sf-border);
        text-transform: uppercase;
        letter-spacing: 0.07em;
        font-size: 10.5px;
        background: rgba(0, 0, 0, 0.15);
        white-space: nowrap;
    }

    .session-row {
        border-bottom: 1px solid var(--sf-border-soft);
        transition: background 0.15s ease;
    }

    .session-row:last-child {
        border-bottom: none;
    }

    .session-row:hover {
        background: var(--sf-row-hover);
    }

    .session-row td {
        padding: 14px 20px;
        vertical-align: middle;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* Host column */
    .host-main {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .host-dot {
        width: 7px;
        height: 7px;
        border-radius: 50%;
        background: var(--sf-success);
        box-shadow: 0 0 6px var(--sf-success);
        flex-shrink: 0;
    }

    .host-label {
        font-weight: 600;
        color: var(--sf-text);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* Address column */
    .td-address code {
        background: rgba(41, 182, 246, 0.08);
        padding: 3px 8px;
        border-radius: 4px;
        color: var(--sf-accent);
        font-family: "JetBrains Mono", "Fira Code", monospace;
        font-size: 12px;
        border: 1px solid rgba(41, 182, 246, 0.15);
        display: inline-block;
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* Time column */
    .td-time {
        color: var(--sf-text-dim);
        font-size: 12px;
    }

    /* Actions column */
    .td-actions {
        text-align: right;
    }

    .action-group {
        display: inline-flex;
        align-items: center;
        justify-content: flex-end;
        gap: 6px;
    }

    .icon-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid var(--sf-border);
        color: var(--sf-text-dim);
        width: 32px;
        height: 32px;
        border-radius: var(--radius-sm);
        cursor: pointer;
        transition:
            color 0.15s,
            border-color 0.15s,
            background 0.15s,
            box-shadow 0.15s;
        flex-shrink: 0;
    }

    .icon-btn.user:hover {
        color: #a78bfa;
        border-color: #a78bfa;
        background: rgba(167, 139, 250, 0.1);
        box-shadow: 0 0 10px rgba(167, 139, 250, 0.15);
    }

    .icon-btn.sftp:hover {
        color: var(--sf-accent);
        border-color: var(--sf-accent);
        background: var(--sf-accent-dim);
        box-shadow: 0 0 10px var(--sf-accent-glow);
    }

    .icon-btn.delete:hover {
        color: var(--sf-error);
        border-color: var(--sf-error);
        background: rgba(240, 83, 83, 0.1);
        box-shadow: 0 0 10px rgba(240, 83, 83, 0.15);
    }

    /* Skeleton */
    .skeleton-row td {
        padding: 14px 20px;
    }

    .shimmer {
        height: 18px;
        border-radius: 4px;
        background: linear-gradient(
            90deg,
            var(--sf-border-soft) 25%,
            #1e2d40 50%,
            var(--sf-border-soft) 75%
        );
        background-size: 200% 100%;
        animation: shimmer 1.4s infinite;
    }

    @keyframes shimmer {
        0% {
            background-position: 200% 0;
        }
        100% {
            background-position: -200% 0;
        }
    }

    /* Empty state */
    .empty-state {
        padding: 48px 20px !important;
        text-align: center;
    }

    .empty-inner {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 10px;
        color: var(--sf-text-muted);
        font-size: 13px;
    }

    /* ── Loading Overlay ─────────────────────────────────────────────────── */
    .glass-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.6);
        backdrop-filter: blur(10px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .loader-box {
        text-align: center;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 14px;
    }

    .spinner {
        width: 42px;
        height: 42px;
        border: 2.5px solid rgba(255, 255, 255, 0.08);
        border-top-color: var(--sf-accent);
        border-radius: 50%;
        animation: spin 0.9s linear infinite;
    }

    .loader-status {
        font-size: 13px;
        color: var(--sf-text-dim);
        margin: 0;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    /* ── Toast Notifications ─────────────────────────────────────────────── */
    .toast-container {
        position: fixed;
        bottom: 24px;
        right: 24px;
        z-index: 9999;
        display: flex;
        flex-direction: column;
        gap: 10px;
        pointer-events: none;
    }

    .toast {
        position: relative;
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 12px 14px 14px 14px;
        border-radius: var(--radius-md);
        min-width: 280px;
        max-width: 400px;
        font-size: 13px;
        font-weight: 500;
        pointer-events: all;
        overflow: hidden;
        backdrop-filter: blur(12px);
        animation: toast-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
        box-shadow:
            0 8px 32px rgba(0, 0, 0, 0.4),
            0 1px 0 rgba(255, 255, 255, 0.04) inset;
    }

    @keyframes toast-in {
        from {
            opacity: 0;
            transform: translateX(20px) scale(0.96);
        }
        to {
            opacity: 1;
            transform: translateX(0) scale(1);
        }
    }

    .toast-error {
        background: rgba(240, 83, 83, 0.1);
        border: 1px solid rgba(240, 83, 83, 0.3);
        color: #fca5a5;
    }

    .toast-success {
        background: rgba(38, 194, 129, 0.1);
        border: 1px solid rgba(38, 194, 129, 0.3);
        color: #6ee7b7;
    }

    .toast-info {
        background: rgba(41, 182, 246, 0.08);
        border: 1px solid rgba(41, 182, 246, 0.25);
        color: #93c5fd;
    }

    .toast-snowflake {
        flex-shrink: 0;
        opacity: 0.9;
    }

    .toast-error .toast-snowflake {
        color: var(--sf-error);
    }
    .toast-success .toast-snowflake {
        color: var(--sf-success);
    }
    .toast-info .toast-snowflake {
        color: var(--sf-accent);
    }

    .toast-msg {
        flex: 1;
        line-height: 1.4;
        word-break: break-word;
    }

    .toast-close {
        flex-shrink: 0;
        background: none;
        border: none;
        color: inherit;
        opacity: 0.5;
        cursor: pointer;
        padding: 2px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        transition: opacity 0.15s;
    }

    .toast-close:hover {
        opacity: 1;
    }

    /* Progress bar at bottom of toast */
    .toast-progress {
        position: absolute;
        bottom: 0;
        left: 0;
        height: 2px;
        width: 100%;
        animation: progress-drain 4.5s linear forwards;
        transform-origin: left;
    }

    .toast-error .toast-progress {
        background: var(--sf-error);
    }
    .toast-success .toast-progress {
        background: var(--sf-success);
    }
    .toast-info .toast-progress {
        background: var(--sf-accent);
    }

    @keyframes progress-drain {
        from {
            transform: scaleX(1);
        }
        to {
            transform: scaleX(0);
        }
    }

    /* ── Session Info Modal ──────────────────────────────────────────────── */
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.65);
        backdrop-filter: blur(8px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
        padding: 20px;
    }

    .modal-card {
        background: var(--sf-bg-card);
        border: 1px solid var(--sf-border);
        border-radius: var(--radius-lg);
        width: 100%;
        max-width: 480px;
        box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
        overflow: hidden;
        animation: modal-in 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
    }

    @keyframes modal-in {
        from {
            opacity: 0;
            transform: scale(0.94) translateY(12px);
        }
        to {
            opacity: 1;
            transform: scale(1) translateY(0);
        }
    }

    .modal-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        padding: 20px 20px 0;
    }

    .modal-title-row {
        display: flex;
        align-items: center;
        gap: 12px;
    }

    .modal-icon {
        width: 36px;
        height: 36px;
        border-radius: 8px;
        background: rgba(167, 139, 250, 0.12);
        border: 1px solid rgba(167, 139, 250, 0.25);
        color: #a78bfa;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
    }

    .modal-title {
        font-size: 15px;
        font-weight: 700;
        color: var(--sf-text);
        margin: 0;
        line-height: 1.2;
    }

    .modal-subtitle {
        font-size: 12px;
        color: var(--sf-text-dim);
        margin: 2px 0 0;
    }

    .modal-close-btn {
        background: none;
        border: none;
        color: var(--sf-text-dim);
        cursor: pointer;
        padding: 4px;
        border-radius: 4px;
        display: flex;
        align-items: center;
        transition: color 0.15s;
        margin-top: 2px;
    }

    .modal-close-btn:hover {
        color: var(--sf-text);
    }

    /* Tabs */
    .modal-tabs {
        display: flex;
        gap: 0;
        padding: 16px 20px 0;
        border-bottom: 1px solid var(--sf-border);
        margin-top: 16px;
    }

    .modal-tab {
        padding: 8px 16px;
        font-size: 12.5px;
        font-weight: 600;
        color: var(--sf-text-dim);
        background: none;
        border: none;
        border-bottom: 2px solid transparent;
        cursor: pointer;
        transition:
            color 0.15s,
            border-color 0.15s;
        margin-bottom: -1px;
        letter-spacing: 0.01em;
    }

    .modal-tab:hover {
        color: var(--sf-text);
    }

    .modal-tab.active {
        color: var(--sf-accent);
        border-bottom-color: var(--sf-accent);
    }

    /* Body */
    .modal-body {
        padding: 20px;
    }

    /* Info grid */
    .info-grid {
        display: flex;
        flex-direction: column;
        gap: 1px;
        background: var(--sf-border-soft);
        border: 1px solid var(--sf-border-soft);
        border-radius: var(--radius-sm);
        overflow: hidden;
        margin-bottom: 14px;
    }

    .info-row {
        display: flex;
        align-items: center;
        gap: 12px;
        background: var(--sf-bg-side);
        padding: 10px 14px;
    }

    .info-label {
        font-size: 11.5px;
        font-weight: 600;
        color: var(--sf-text-muted);
        text-transform: uppercase;
        letter-spacing: 0.06em;
        width: 110px;
        flex-shrink: 0;
    }

    .info-value {
        font-size: 13px;
        color: var(--sf-text);
    }

    .info-code {
        font-family: "JetBrains Mono", monospace;
        font-size: 12px;
        color: var(--sf-accent);
        background: rgba(41, 182, 246, 0.07);
        padding: 2px 7px;
        border-radius: 4px;
        border: 1px solid rgba(41, 182, 246, 0.12);
    }

    .mono-small {
        font-size: 11px;
    }

    .info-readonly-note {
        font-size: 11.5px;
        color: var(--sf-text-muted);
        display: flex;
        align-items: center;
        gap: 6px;
        margin: 0;
    }

    .info-readonly-note strong {
        color: var(--sf-text-dim);
    }

    /* Custom SFTP tab */
    .sftp-tab-desc {
        font-size: 12.5px;
        color: var(--sf-text-dim);
        margin: 0 0 18px;
        line-height: 1.5;
    }

    .form-group {
        margin-bottom: 14px;
    }

    .form-label {
        display: block;
        font-size: 11.5px;
        font-weight: 600;
        color: var(--sf-text-dim);
        text-transform: uppercase;
        letter-spacing: 0.06em;
        margin-bottom: 7px;
    }

    .optional {
        font-weight: 400;
        text-transform: none;
        letter-spacing: 0;
        color: var(--sf-text-muted);
    }

    .form-input {
        width: 100%;
        background: var(--sf-bg-side);
        border: 1px solid var(--sf-border);
        border-radius: var(--radius-sm);
        color: var(--sf-text);
        font-size: 13px;
        padding: 9px 12px;
        outline: none;
        transition:
            border-color 0.15s,
            box-shadow 0.15s;
        box-sizing: border-box;
        font-family: inherit;
    }

    .form-input::placeholder {
        color: var(--sf-text-muted);
    }

    .form-input:focus {
        border-color: var(--sf-accent);
        box-shadow: 0 0 0 3px var(--sf-accent-dim);
    }

    .btn-connect {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        width: 100%;
        padding: 10px 16px;
        background: var(--sf-accent);
        color: #0b0f14;
        font-weight: 700;
        font-size: 13px;
        border: none;
        border-radius: var(--radius-sm);
        cursor: pointer;
        transition:
            opacity 0.15s,
            box-shadow 0.15s;
        margin-top: 8px;
        letter-spacing: 0.02em;
    }

    .btn-connect:hover:not(:disabled) {
        opacity: 0.9;
        box-shadow: 0 4px 16px var(--sf-accent-glow);
    }

    .btn-connect:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .btn-spinner {
        width: 14px;
        height: 14px;
        border: 2px solid rgba(0, 0, 0, 0.2);
        border-top-color: rgba(0, 0, 0, 0.7);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }
</style>
