<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { listen } from "@tauri-apps/api/event";
    import { Menu } from "@tauri-apps/api/menu";

    type SessionStatus = "connected" | "connecting" | "disconnected" | "error";

    export let id: string;
    export let label: string = "New Session";
    export let status: SessionStatus = "disconnected";
    export let kind: "ssh" | "sftp" | "guacamole" = "ssh";
    export let active: boolean = false;
    export let hasActivity: boolean = false;

    export let onselect: (id: string) => void = () => {};
    export let onclose: (id: string) => Promise<void> = async () => {};
    export let onduplicate: (id: string) => Promise<void> = async () => {};

    let menuPromise : any;
    let unlistenPromise : any;

    function handleClick(): void {
        onselect(id);
    }

    function handleClose(e: MouseEvent | KeyboardEvent): void {
        e.stopPropagation();
        onclose(id);
    }

    function handleKeydown(e: KeyboardEvent): void {
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleClick();
        }
        if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault();
            onclose(id);
        }
    }

    function handleDuplicate(){
        onduplicate(id)
    }


    // Pas status "connected", warna dotnya dibedain per protocol (ssh/sftp/guacamole)
    // supaya kelihatan bedanya di tab bar. Status connecting/error/disconnected tetep
    // pake warna semantic biasa.
    $: kindColor =
        kind === "sftp"
            ? "var(--sf-accent, #4fc3f7)"
            : kind === "guacamole"
              ? "#a78bfa"
              : "var(--sf-status-online)";

    $: statusColor =
        status === "connected"
            ? kindColor
            : status === "connecting"
              ? "var(--sf-status-warning)"
              : status === "error"
                ? "var(--sf-status-error)"
                : "var(--sf-status-offline)";

    $: statusLabel =
        status === "connected"
            ? "connected"
            : status === "connecting"
              ? "connecting…"
              : status === "error"
                ? "error"
                : "offline";


    onMount(() => {
        // 1. Initialize the native menu
        menuPromise = Menu.new({
        items: [
            { id: "session_menu_duplicate", text: "Duplicate", action: handleDuplicate },
            { id: "session_menu_close", text: "Close", action: () => handleClose(undefined) }
        ],
        });

        // 2. Listen for menu selection events globally
        unlistenPromise = listen("menu-event", (event) => {
        const payload = event.payload;
        
        // Filter events by prefix to ensure they belong to this context menu
        if (typeof payload === "string" && payload.startsWith("ctx")) {
            switch (payload) {
            case "ctx_option1":
                console.log("Option 1 clicked");
                break;
            case "ctx_option2":
                console.log("Option 2 clicked");
                break;
            default:
                console.log("Unimplemented menu id:", payload);
            }
        }
        });
    });       
    
    // 3. Cleanup the event listener on component destroy
    onDestroy(async () => {
        if (unlistenPromise) {
        const unlisten = await unlistenPromise;
        unlisten();
        }
    });

    async function handleContextMenu(event : MouseEvent) {
        console.log()
        event.preventDefault(); // Prevent the default browser context menu
        const menu = await menuPromise;
        menu.popup(); // Trigger the Tauri native popup
    }

</script>

<button
    class="session-tab"
    class:active
    class:status-connected={status === "connected"}
    class:status-connecting={status === "connecting"}
    class:status-error={status === "error"}
    class:status-disconnected={status === "disconnected"}
    role="tab"
    aria-selected={active}
    aria-label="{label} — {statusLabel}"
    title="{label} — {statusLabel}"
    on:click={handleClick}
    on:keydown={handleKeydown}
    on:contextmenu={handleContextMenu}
>
    <!-- Status dot with pulse for connecting -->
    <span
        class="status-dot"
        aria-hidden="true"
        style="background: {statusColor};"
    >
        {#if status === "connecting"}
            <span class="pulse-ring" style="border-color: {statusColor};"
            ></span>
        {/if}
    </span>

    <!-- Tab content -->
    <span class="tab-body">
        <span class="tab-label">
            <span class="kind-icon" class:kind-sftp={kind === "sftp"} class:kind-guacamole={kind === "guacamole"} aria-hidden="true">
                {#if kind === "sftp"}
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                    </svg>
                {:else if kind === "guacamole"}
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <rect x="2" y="3" width="20" height="14" rx="2" />
                        <line x1="8" y1="21" x2="16" y2="21" />
                        <line x1="12" y1="17" x2="12" y2="21" />
                    </svg>
                {:else}
                    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="4 17 10 11 4 5" />
                        <line x1="12" y1="19" x2="20" y2="19" />
                    </svg>
                {/if}
            </span>
            {label}
        </span>
    </span>

    <!-- Activity indicator -->
    {#if hasActivity && !active}
        <span class="activity-dot" aria-label="New output" aria-hidden="true"
        ></span>
    {/if}

    <!-- Close button -->
    <span
        class="tab-close"
        role="button"
        tabindex="-1"
        aria-label="Close {label}"
        on:click={handleClose}
        on:keydown={(e) => e.key === "Enter" && handleClose(e)}
    >
        <svg
            width="8"
            height="8"
            viewBox="0 0 8 8"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            aria-hidden="true"
        >
            <line x1="1" y1="1" x2="7" y2="7" />
            <line x1="7" y1="1" x2="1" y2="7" />
        </svg>
    </span>
</button>

<style>
    .session-tab {
        position: relative;
        display: flex;
        align-items: center;
        gap: 7px;
        padding: 0 10px 0 10px;
        height: var(--sf-tabbar-height, 38px);
        min-width: 120px;
        max-width: 200px;
        cursor: pointer;
        border: none;
        border-bottom: 2px solid transparent;
        border-right: 0.5px solid var(--sf-border);
        background: transparent;
        font-family: var(--sf-font-ui, "Inter", sans-serif);
        font-size: 12px;
        color: var(--sf-text-secondary);
        white-space: nowrap;
        text-align: left;
        transition:
            background 0.15s ease,
            color 0.15s ease,
            border-color 0.15s ease;
        user-select: none;
        flex-shrink: 0;
        outline: none;
    }

    .session-tab:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-text-primary);
    }

    .session-tab:focus-visible {
        box-shadow: inset 0 0 0 1.5px var(--sf-border-focus);
    }

    .session-tab.active {
        background: var(--sf-bg-surface);
        color: var(--sf-text-primary);
        border-bottom-color: var(--sf-accent);
    }

    .session-tab.active.status-error {
        border-bottom-color: var(--sf-status-error);
    }
    .session-tab.active.status-connecting {
        border-bottom-color: var(--sf-status-warning);
    }

    .status-dot {
        position: relative;
        width: 7px;
        height: 7px;
        border-radius: 50%;
        flex-shrink: 0;
        transition: background 0.2s;
    }

    .pulse-ring {
        position: absolute;
        inset: -3px;
        border-radius: 50%;
        border: 1.5px solid;
        opacity: 0;
        animation: pulse 1.4s ease-out infinite;
    }

    @keyframes pulse {
        0% {
            transform: scale(0.8);
            opacity: 0.8;
        }
        100% {
            transform: scale(2.2);
            opacity: 0;
        }
    }

    .tab-body {
        display: flex;
        flex-direction: column;
        min-width: 0;
        flex: 1;
        gap: 1px;
    }

    .tab-label {
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 12px;
        font-weight: 500;
        overflow: hidden;
        text-overflow: ellipsis;
        line-height: 1.3;
        color: inherit;
        letter-spacing: 0.01em;
    }

    .kind-icon {
        display: flex;
        align-items: center;
        flex-shrink: 0;
        opacity: 0.75;
    }

    .kind-icon.kind-sftp {
        color: var(--sf-accent, #4fc3f7);
    }

    .kind-icon.kind-guacamole {
        color: #a78bfa;
    }

    .activity-dot {
        width: 5px;
        height: 5px;
        border-radius: 50%;
        background: var(--sf-accent);
        flex-shrink: 0;
        animation: blink-activity 2s ease-in-out infinite;
    }

    @keyframes blink-activity {
        0%,
        100% {
            opacity: 1;
        }
        50% {
            opacity: 0.3;
        }
    }

    .tab-close {
        width: 18px;
        height: 18px;
        border-radius: var(--sf-radius-sm, 6px);
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--sf-text-hint);
        cursor: pointer;
        flex-shrink: 0;
        opacity: 0;
        transition:
            background 0.1s,
            color 0.1s,
            opacity 0.15s;
    }

    .session-tab:hover .tab-close,
    .session-tab.active .tab-close {
        opacity: 1;
    }

    .tab-close:hover {
        background: rgba(239, 83, 80, 0.15);
        color: var(--sf-status-error);
    }
</style>
