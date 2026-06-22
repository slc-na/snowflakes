<script lang="ts">
    import { onDestroy, onMount, tick } from "svelte";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import SessionTab from "./SessionTab.svelte";
    import { page } from "$app/state";
    import { connectToSession, deleteSession } from "../../controller/ssh";
    import { disconnectSftp } from "../../controller/sftp";
    import ErrorModal from "../modal/ErrorModal.svelte";
    import { loadSessionInfo } from "../../controller/local";
    import { loadSessionPass } from "../../controller/vault";

    type SessionStatus = "connected" | "connecting" | "disconnected" | "error";
    type SessionKind = "ssh" | "sftp";

    type Session = {
        id: string;
        label: string;
        status: SessionStatus;
        hasActivity: boolean;
        kind: SessionKind;
    };

    export let showAddButton: boolean = false;

    let sessions: Session[] = [];
    let activeId: string | null = null;
    let tabBarEl: HTMLDivElement;
    let overflowLeft: boolean = false;
    let overflowRight: boolean = false;
    let unlistenSessionUpdated: UnlistenFn | null = null;
    let connectingStatus = ""

    $: activeSession = sessions.find((s) => s.id === activeId) ?? null;
    $: connectedCount = sessions.filter((s) => s.status === "connected").length;
    $: errorCount = sessions.filter((s) => s.status === "error").length;

    let unlistenSftpSessionUpdated: UnlistenFn | null = null;

    async function fetchSessions(
        activeNow: string | null = null,
    ): Promise<void> {
        try {
            const [sshKeys, sftpKeys] = await Promise.all([
                invoke<string[]>("get_active_session"),
                invoke<string[]>("get_active_sftp_session").catch(() => []),
            ]);

            const existingIds = new Set(sessions.map((s) => s.id));
            const newIds = new Set([...sshKeys, ...sftpKeys]);

            for (const key of sshKeys) {
                if (!existingIds.has(key)) {
                    sessions = [
                        ...sessions,
                        {
                            id: key,
                            label: key,
                            status: "connected",
                            hasActivity: false,
                            kind: "ssh",
                        },
                    ];
                }
            }

            for (const key of sftpKeys) {
                if (!existingIds.has(key)) {
                    sessions = [
                        ...sessions,
                        {
                            id: key,
                            label: key,
                            status: "connected",
                            hasActivity: false,
                            kind: "sftp",
                        },
                    ];
                }
            }

            sessions = sessions.filter((s) => newIds.has(s.id));
        } catch (err) {
            console.error("[SessionTabBar] fetchSessions error:", err);
        }
    }

    onMount(async () => {
        fetchSessions();
        unlistenSessionUpdated = await listen(
            "session_updated",
            (activeNow) => {
                fetchSessions();
                console.log(activeNow);
                activeId = activeNow.payload as string;
            },
        );
        unlistenSftpSessionUpdated = await listen("sftp_session_updated", () => {
            fetchSessions();
        });
    });

    onDestroy(() => {
        if (unlistenSessionUpdated) unlistenSessionUpdated();
        if (unlistenSftpSessionUpdated) unlistenSftpSessionUpdated();
    });

    function selectSession(id: string): void {
        const session = sessions.find((s) => s.id === id);
        activeId = id;
        tick().then(() => scrollTabIntoView(id));
        if (session?.kind === "sftp") {
            goto(`/files?key=${encodeURIComponent(id)}`);
        } else {
            goto(`/session?key=${encodeURIComponent(id)}`);
        }
    }

    async function duplicateSession(id: string): Promise<void> {
        const session = sessions.find((s) => s.id === id);
        if (session?.kind === "sftp") {
            // SFTP sessions don't have a duplicate flow yet
            return;
        }

        let sessionInfo = await loadSessionInfo(id)
        if(!sessionInfo) {
            throw new Error("Session info not found")
        }

        const password = await loadSessionPass(sessionInfo.sessionKey);
        if(!password){
            throw new Error("Session password not found")
        }

        sessionInfo.password = password
        connectToSession(
            sessionInfo,
            (msg) => {
                connectingStatus = msg;
            }
        )
    

        // TODO

        // activeId = id;
        // tick().then(() => scrollTabIntoView(id));
        // console.log(`/session?key=${encodeURIComponent(id)}`);
        // goto(`/session?key=${encodeURIComponent(id)}`);
    }

    async function closeSession(id: string): Promise<void> {
        const session = sessions.find((s) => s.id === id);
        try {
            if (session?.kind === "sftp") {
                await disconnectSftp(id);
            } else {
                await deleteSession(id);
            }
        } catch (err) {
            console.warn("[SessionTabBar] disconnect error:", err);
        }

        goto("/");
    }

    function checkOverflow(): void {
        if (!tabBarEl) return;
        overflowLeft = tabBarEl.scrollLeft > 4;
        overflowRight =
            tabBarEl.scrollLeft + tabBarEl.clientWidth <
            tabBarEl.scrollWidth - 4;
    }

    function scrollTabIntoView(id: string): void {
        if (!tabBarEl) return;
        const el = tabBarEl.querySelector(`[data-sid="${id}"]`);
        el?.scrollIntoView({
            behavior: "smooth",
            block: "nearest",
            inline: "nearest",
        });
    }

    function scrollLeft(): void {
        tabBarEl?.scrollBy({ left: -160, behavior: "smooth" });
    }
    function scrollRight(): void {
        tabBarEl?.scrollBy({ left: 160, behavior: "smooth" });
    }

    function isTerminalFocused(): boolean {
        const active = document.activeElement;
        return !!active?.closest(".xterm");
    }

    function handleKeydown(e: KeyboardEvent): void {
        if (!e.ctrlKey) return;

        // Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+T only take effect once the terminal
        // has been blurred (via Esc on the session page) - while it's focused,
        // these combos are left alone instead of fighting xterm's own key handling.
        if (
            (e.key === "Tab" || e.key === "t" || e.key === "T") &&
            isTerminalFocused()
        ) {
            return;
        }

        if (e.key === "Tab") {
            e.preventDefault();
            const idx = sessions.findIndex((s) => s.id === activeId);
            const next = e.shiftKey
                ? sessions[(idx - 1 + sessions.length) % sessions.length]
                : sessions[(idx + 1) % sessions.length];
            if (next) selectSession(next.id);
        }
        if (e.key === "w" && activeId !== null) {
            e.preventDefault();
            closeSession(activeId);
        }
        if (e.key === "t" || e.key === "T") {
            e.preventDefault();
            goto("/");
        }
    }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="tab-bar-wrapper" role="tablist" aria-label="SSH sessions">
    <!-- Left scroll arrow -->
    {#if overflowLeft}
        <button
            class="scroll-btn scroll-left"
            aria-label="Scroll left"
            on:click={scrollLeft}
            tabindex="-1"
        >
            <svg
                width="11"
                height="11"
                viewBox="0 0 11 11"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                aria-hidden="true"
            >
                <polyline points="7,1 3,5.5 7,10" />
            </svg>
        </button>
    {/if}

    <!-- Scrollable tab strip -->
    <div class="tab-strip" bind:this={tabBarEl} on:scroll={checkOverflow}>
        {#each sessions as session (session.id)}
            <div data-sid={session.id}>
                <SessionTab
                    id={session.id}
                    label={session.label}
                    status={session.status}
                    kind={session.kind}
                    active={session.id === activeId}
                    hasActivity={session.hasActivity ?? false}
                    onselect={selectSession}
                    onclose={closeSession}
                    onduplicate={duplicateSession}
                />
            </div>
        {:else}
            <span class="no-sessions">No active sessions</span>
        {/each}
    </div>

    <!-- Right scroll arrow -->
    {#if overflowRight}
        <button
            class="scroll-btn scroll-right"
            aria-label="Scroll right"
            on:click={scrollRight}
            tabindex="-1"
        >
            <svg
                width="11"
                height="11"
                viewBox="0 0 11 11"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                aria-hidden="true"
            >
                <polyline points="4,1 8,5.5 4,10" />
            </svg>
        </button>
    {/if}

    <!-- Right-side actions -->
    <div class="tab-actions">
        <!-- Connection count pills -->
        <div class="status-summary" aria-label="Connection summary">
            {#if connectedCount > 0}
                <span
                    class="summary-pill pill-online"
                    title="{connectedCount} connected"
                >
                    <span class="pill-dot" aria-hidden="true"
                    ></span>{connectedCount}
                </span>
            {/if}
            {#if errorCount > 0}
                <span
                    class="summary-pill pill-error"
                    title="{errorCount} error"
                >
                    <span class="pill-dot" aria-hidden="true"
                    ></span>{errorCount}
                </span>
            {/if}
        </div>

        <!-- Active session key chip -->
        {#if activeSession}
            <div
                class="host-chip"
                aria-label="Active session: {activeSession.id}"
            >
                <svg
                    width="10"
                    height="10"
                    viewBox="0 0 10 10"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.4"
                    aria-hidden="true"
                >
                    <rect x="1" y="1" width="8" height="8" rx="1.5" />
                    <line x1="3" y1="3.8" x2="7" y2="3.8" />
                    <line x1="3" y1="6" x2="5.5" y2="6" />
                </svg>
                <span class="host-chip-text">{activeSession.id}</span>
            </div>
        {/if}

        <!-- New session button (hidden by default; sessions come from backend) -->
        {#if showAddButton}
            <button
                class="add-btn"
                title="Refresh sessions"
                aria-label="Refresh sessions"
                on:click={() => {
                    fetchSessions();
                }}
            >
                <svg
                    width="12"
                    height="12"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    aria-hidden="true"
                >
                    <line x1="6" y1="1" x2="6" y2="11" />
                    <line x1="1" y1="6" x2="11" y2="6" />
                </svg>
            </button>
        {/if}
    </div>
</div>

<style>
    .tab-bar-wrapper {
        position: relative;
        display: flex;
        align-items: stretch;
        height: var(--sf-tabbar-height, 38px);
        background: var(--sf-bg-tab-bar, #0b1929);
        border-bottom: 1px solid var(--sf-border, #1a3352);
        flex-shrink: 0;
        overflow: hidden;
    }

    /* ── Scrollable strip ───────────────────────────────── */
    .tab-strip {
        display: flex;
        align-items: stretch;
        flex: 1;
        overflow-x: auto;
        overflow-y: hidden;
        scrollbar-width: none;
    }
    .tab-strip::-webkit-scrollbar {
        display: none;
    }

    /* ── Empty state ────────────────────────────────────── */
    .no-sessions {
        display: flex;
        align-items: center;
        padding: 0 16px;
        font-size: 11px;
        font-family: var(--sf-font-ui, "Inter", sans-serif);
        color: var(--sf-text-hint, #4a6a8a);
        white-space: nowrap;
        opacity: 0.7;
        font-style: italic;
    }

    /* ── Scroll arrows ──────────────────────────────────── */
    .scroll-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 24px;
        flex-shrink: 0;
        border: none;
        background: var(--sf-bg-tab-bar);
        color: var(--sf-text-secondary);
        cursor: pointer;
        transition:
            color 0.13s,
            background 0.13s;
        z-index: 2;
    }
    .scroll-btn:hover {
        color: var(--sf-text-primary);
        background: var(--sf-bg-hover);
    }
    .scroll-left {
        border-right: 0.5px solid var(--sf-border);
    }
    .scroll-right {
        border-left: 0.5px solid var(--sf-border);
    }

    /* ── Action area ────────────────────────────────────── */
    .tab-actions {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 0 10px;
        border-left: 0.5px solid var(--sf-border);
        flex-shrink: 0;
    }

    /* ── Status pills ───────────────────────────────────── */
    .status-summary {
        display: flex;
        align-items: center;
        gap: 4px;
    }

    .summary-pill {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 10px;
        font-weight: 500;
        font-family: var(--sf-font-ui, "Inter", sans-serif);
        padding: 2px 7px;
        border-radius: var(--sf-radius-pill, 999px);
        line-height: 1.5;
        user-select: none;
    }
    .pill-online {
        background: rgba(38, 198, 160, 0.12);
        color: var(--sf-status-online, #26c6a0);
        border: 0.5px solid rgba(38, 198, 160, 0.28);
    }
    .pill-error {
        background: rgba(239, 83, 80, 0.12);
        color: var(--sf-status-error, #ef5350);
        border: 0.5px solid rgba(239, 83, 80, 0.28);
    }
    .pill-dot {
        width: 5px;
        height: 5px;
        border-radius: 50%;
        background: currentColor;
        flex-shrink: 0;
    }

    /* ── Host chip ──────────────────────────────────────── */
    .host-chip {
        display: flex;
        align-items: center;
        gap: 5px;
        padding: 3px 9px;
        border-radius: var(--sf-radius-pill, 999px);
        border: 0.5px solid var(--sf-border, #1a3352);
        background: rgba(79, 195, 247, 0.05);
        color: var(--sf-text-secondary);
        max-width: 180px;
        overflow: hidden;
    }
    .host-chip-text {
        font-size: 10px;
        font-family: var(--sf-font-mono, monospace);
        color: var(--sf-text-accent, #4fc3f7);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* ── Add button ─────────────────────────────────────── */
    .add-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 26px;
        height: 26px;
        border-radius: var(--sf-radius-sm, 6px);
        border: 0.5px solid var(--sf-border, #1a3352);
        background: transparent;
        color: var(--sf-text-secondary, #7aa8cc);
        cursor: pointer;
        flex-shrink: 0;
        transition:
            background 0.13s,
            color 0.13s,
            border-color 0.13s;
    }
    .add-btn:hover {
        background: var(--sf-bg-hover, #12253d);
        color: var(--sf-accent, #4fc3f7);
        border-color: var(--sf-border-hover, #2a6a9a);
    }
</style>
