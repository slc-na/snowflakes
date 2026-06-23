<script lang="ts">
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import type { SessionInfo } from "../../types/settings";
    import { loadSessionInfo } from "../../controller/local";
    import {
        getGuacamoleToken,
        disconnectGuacamoleSession,
    } from "../../controller/guacamole";

    const GUACAMOLE_URL = "https://moses.apps.slc.net";

    let targetKey = $derived($page.url.searchParams.get("key") || "unknown");
    let session = $state<SessionInfo | null>(null);
    let iframeEl: HTMLIFrameElement;

    function attachAuthToken(): void {
        const token = getGuacamoleToken(targetKey);
        if (!token || !iframeEl?.contentWindow) return;

        try {
            iframeEl.contentWindow.localStorage.setItem("GUAC_AUTH_TOKEN", token);
            iframeEl.contentWindow.location.reload();
        } catch (e) {
            console.warn(
                "[Guacamole] Failed to inject auth token into iframe (likely blocked by cross-origin policy):",
                e,
            );
        }
    }

    $effect(() => {
        if (targetKey && targetKey !== "unknown") {
            loadSessionInfo(targetKey).then((info) => (session = info));
        }
    });

    async function handleDisconnect() {
        disconnectGuacamoleSession(targetKey);
        goto("/");
    }
</script>

<div class="flex flex-col h-screen bg-[#1a1b26]">
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

    <div class="flex-1">
        <iframe
            bind:this={iframeEl}
            src={GUACAMOLE_URL}
            title="Guacamole Remote Desktop"
            class="w-full h-full border-0"
            onload={attachAuthToken}
        ></iframe>
    </div>
</div>
