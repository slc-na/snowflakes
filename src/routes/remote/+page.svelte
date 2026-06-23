<script lang="ts">
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import type { SessionInfo } from "../../types/settings";
    import { loadSessionInfo } from "../../controller/local";
    import {
        openGuacamoleWindow,
        focusGuacamoleWindow,
        disconnectGuacamoleSession,
    } from "../../controller/guacamole";

    let targetKey = $derived($page.url.searchParams.get("key") || "unknown");
    let session = $state<SessionInfo | null>(null);
    let errorMsg = $state("");

    // The actual Guacamole connection lives in its own Tauri window (see
    // open_guacamole_window in src-tauri/src/http/guacamole.rs) - an embedded
    // iframe can't have its localStorage written from this page because of the
    // browser's same-origin policy, so there's nothing to render here besides a
    // way to bring that window back to front.
    async function bringToFront() {
        errorMsg = "";
        try {
            const focused = await focusGuacamoleWindow(targetKey);
            if (!focused) {
                await openGuacamoleWindow(targetKey);
            }
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    $effect(() => {
        if (targetKey && targetKey !== "unknown") {
            loadSessionInfo(targetKey).then((info) => (session = info));
            bringToFront();
        }
    });

    async function handleDisconnect() {
        await disconnectGuacamoleSession(targetKey);
        goto("/");
    }
</script>

<div class="flex flex-col h-full bg-[#1a1b26]">
    <div
        class="px-4 py-2 bg-[#16161e] border-b border-[#24283b] flex justify-between items-center"
    >
        <div class="text-[11px] text-[#565f89] font-mono">
            REMOTE DESKTOP: <span class="text-[#7aa2f7]"
                >{session?.label || targetKey}</span
            >
            {#if session}
                <span class="mx-2 opacity-30">|</span>
                <span class="opacity-70"
                    >{session.username}@{session.targetIp}</span
                >
            {/if}
        </div>
        <button
            class="text-[11px] text-[#f7768e] hover:underline"
            onclick={handleDisconnect}
        >
            Disconnect
        </button>
    </div>

    <div class="flex-1 flex flex-col items-center justify-center gap-4 text-[#565f89]">
        <p class="text-sm">
            This remote desktop is open in its own window.
        </p>
        <button
            class="text-[12px] px-4 py-2 rounded border border-[#24283b] text-[#7aa2f7] hover:bg-[#1f2335]"
            onclick={bringToFront}
        >
            Bring window to front
        </button>
        {#if errorMsg}
            <p class="text-[11px] text-[#f7768e]">{errorMsg}</p>
        {/if}
    </div>
</div>
