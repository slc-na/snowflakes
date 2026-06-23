<script lang="ts">
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { loadDefaultAccount } from "../../controller/vault";
    import { connectToSession } from "../../controller/ssh";
    import type { SnowflakesSettings, SessionInfo } from "../../types/settings";
    import { DEFAULT_SETTINGS } from "../../types/settings";
    import { loadSettings } from "../../controller/local";

    let hostname = $state("");
    let port = $state("22");
    let username = $state("");
    let password = $state("");
    let label = $state("");
    let showPassword = $state(false);
    let bastionIp = $state("");
    let sshTemplate = $state(DEFAULT_SETTINGS.sshTemplate);
    let errorMsg = $state("");
    let isConnecting = $state(false);
    let connectingStatus = $state("");

    let {
        isOpen,
        onClose,
        prefill = null,
    } = $props<{
        isOpen: boolean;
        onClose: () => void;
        prefill?: SessionInfo | null;
    }>();

    // Re-populate fields every time the modal opens so server-card prefill works
    $effect(() => {
        if (!isOpen) return;
        (async () => {
            try {
                const settings = await loadSettings();
                bastionIp = settings.bastionIp;
                sshTemplate = settings.sshTemplate;

                const account = await loadDefaultAccount();

                // Reset fields first
                hostname = "";
                port = "22";
                username = "";
                password = "";
                label = "";
                errorMsg = "";
                isConnecting = false;
                connectingStatus = "";

                if (prefill) {
                    hostname = prefill.targetIp;
                    port = prefill.port ? String(prefill.port) : "22";
                    username = prefill.username || (account.username ?? "");
                    password = prefill.password || (account.password ?? "");
                    label = prefill.label;
                    bastionIp = prefill.bastionIp || bastionIp;
                } else {
                    if (account.username) username = account.username;
                    if (account.password) password = account.password;
                }
            } catch (err) {
                console.error("[NewHostModal] failed to load settings:", err);
            }
        })();
    });

    let sshCommand = $derived(
        sshTemplate
            .replace(/\{username\}/g, username || "user")
            .replace(/\{bastion\}/g, bastionIp || "bastion")
            .replace(/\{target\}/g, hostname || "hostname"),
    );

    async function handleSubmit() {
        errorMsg = "";
        isConnecting = true;

        try {
            const info: SessionInfo = {
                sessionKey: prefill?.sessionKey || `${hostname}-${Date.now()}`,
                username,
                password,
                targetIp: hostname,
                port: parseInt(port, 10) || 22,
                bastionIp: bastionIp,
                label: label || hostname,
                connectedAt: Date.now(),
            };

            await connectToSession(info, (msg) => {
                connectingStatus = msg;
            });
        } catch (e) {
            console.error("[NewHostModal] SSH start error:", e);
            errorMsg = String(e);
            isConnecting = false;
        }
    }
</script>

{#if isOpen}
    <div class="overlay" role="dialog">
        <div class="modal">
            <div class="modal-header">
                <span class="modal-title">New SSH connection</span>
                <button class="close-btn" onclick={onClose}>✕</button>
            </div>

            <div class="modal-body">
                <div class="field">
                    <label for="label">Label</label>
                    <input
                        id="label"
                        type="text"
                        placeholder="My web server"
                        bind:value={label}
                    />
                </div>

                <div class="row">
                    <div class="field" style="flex:1">
                        <label for="hostname">Hostname / IP</label>
                        <input
                            id="hostname"
                            type="text"
                            placeholder="192.168.1.1"
                            bind:value={hostname}
                        />
                    </div>
                    <div class="field" style="width:80px">
                        <label for="port">Port</label>
                        <input id="port" type="text" bind:value={port} />
                    </div>
                </div>

                <div class="field">
                    <label for="username">Username</label>
                    <input
                        id="username"
                        type="text"
                        placeholder="root"
                        bind:value={username}
                    />
                </div>

                <div class="field">
                    <label for="password">Password</label>
                    <div class="input-wrap">
                        <input
                            id="password"
                            type={showPassword ? "text" : "password"}
                            placeholder="••••••••"
                            bind:value={password}
                        />
                        <button
                            class="eye-btn"
                            onclick={() => (showPassword = !showPassword)}
                        >
                            {showPassword ? "hide" : "show"}
                        </button>
                    </div>
                </div>

                <div class="cmd-label">Generated SSH command</div>
                <div class="cmd-box">
                    <span class="cmd-text">{sshCommand}</span>
                    <button
                        class="copy-btn"
                        onclick={() =>
                            navigator.clipboard.writeText(sshCommand)}
                    >
                        copy
                    </button>
                </div>

                {#if errorMsg}
                    <div class="error-msg">{errorMsg}</div>
                {/if}
            </div>

            <div class="modal-footer">
                <button
                    class="btn-ghost"
                    onclick={onClose}
                    disabled={isConnecting}>Cancel</button
                >
                <button
                    class="btn-primary"
                    onclick={handleSubmit}
                    disabled={isConnecting}
                >
                    {#if isConnecting}
                        <div class="spinner-sm"></div>
                        Connecting...
                    {:else}
                        Save & connect
                    {/if}
                </button>
            </div>

            {#if isConnecting}
                <div class="loading-overlay">
                    <div class="spinner"></div>
                    <span class="loading-text">{connectingStatus}</span>
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        margin-top: 32px;
        inset: 0;
        background: rgba(5, 15, 28, 0.85);
        display: flex;
        align-items: center;
        justify-content: center;
        max-height: calc(100% - 32px);
        overflow-y: auto;
        z-index: 50;
    }

    .modal {
        background: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: 12px;
        width: 420px;
        overflow: hidden;
    }

    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 16px 18px 12px;
        border-bottom: 1px solid var(--sf-border);
    }

    .modal-title {
        font-size: 14px;
        font-weight: 500;
        color: var(--sf-text-primary);
    }

    .close-btn {
        background: none;
        border: none;
        color: var(--sf-text-hint);
        cursor: pointer;
        font-size: 14px;
        padding: 2px 6px;
    }

    .close-btn:hover {
        color: var(--sf-text-muted);
    }

    .modal-body {
        padding: 16px 18px;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .row {
        display: flex;
        gap: 8px;
    }

    .field {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    label {
        font-size: 11px;
        color: var(--sf-text-primary);
    }

    input {
        background: var(--sf-bg-secondary);
        border: 1px solid var(--sf-border);
        border-radius: 7px;
        padding: 8px 10px;
        font-size: 12px;
        color: var(--sf-text-primary);
        outline: none;
        width: 100%;
        font-family: var(--sf-font-ui);
    }

    input:focus {
        border-color: var(--sf-accent);
    }

    input::placeholder {
        color: var(--sf-text-hint);
    }

    .input-wrap {
        position: relative;
    }

    .input-wrap input {
        padding-right: 52px;
    }

    .eye-btn {
        position: absolute;
        right: 8px;
        top: 50%;
        transform: translateY(-50%);
        background: none;
        border: none;
        font-size: 10px;
        color: var(--sf-text-hint);
        cursor: pointer;
    }

    .eye-btn:hover {
        color: var(--sf-text-muted);
    }

    .cmd-label {
        font-size: 11px;
        color: var(--sf-text-primary);
    }

    .cmd-box {
        background: var(--sf-bg-terminal);
        border: 1px solid var(--sf-border);
        border-radius: 7px;
        padding: 10px 12px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
    }

    .cmd-text {
        font-family: var(--sf-font-mono);
        font-size: 11px;
        color: var(--sf-accent);
        word-break: break-all;
        flex: 1;
    }

    .copy-btn {
        font-size: 10px;
        color: var(--sf-text-hint);
        background: none;
        border: 1px solid var(--sf-border);
        border-radius: 5px;
        padding: 3px 8px;
        cursor: pointer;
        white-space: nowrap;
    }

    .copy-btn:hover {
        color: var(--sf-accent);
        border-color: var(--sf-border-hover);
    }

    .modal-footer {
        padding: 12px 18px;
        border-top: 1px solid var(--sf-border);
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }

    .btn-ghost {
        background: transparent;
        border: 1px solid var(--sf-border);
        border-radius: 7px;
        padding: 7px 14px;
        font-size: 12px;
        color: var(--sf-text-muted);
        cursor: pointer;
    }

    .btn-ghost:hover {
        border-color: var(--sf-border-hover);
        color: var(--sf-text);
    }

    .btn-primary {
        background: var(--sf-accent);
        border: none;
        border-radius: 7px;
        padding: 7px 14px;
        font-size: 12px;
        font-weight: 500;
        color: var(--sf-text-on-accent);
        cursor: pointer;
    }

    .btn-primary:hover {
        background: var(--sf-accent-hover);
    }

    .error-msg {
        font-family: var(--sf-font-ui);
        font-size: 11px;
        color: var(--sf-status-error);
        background: rgba(239, 83, 80, 0.08);
        border: 1px solid rgba(239, 83, 80, 0.2);
        border-radius: 6px;
        padding: 8px 10px;
        word-break: break-all;
    }

    /* ── Loading ──────────────────────────────────────── */
    .loading-overlay {
        position: absolute;
        inset: 0;
        background: rgba(
            15,
            34,
            54,
            0.9
        ); /* Matching sf-bg-surface with transparency */
        backdrop-filter: blur(4px);
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 16px;
        z-index: 10;
    }

    .loading-text {
        font-size: 13px;
        color: var(--sf-text-primary);
        font-weight: 500;
        animation: pulse 1.5s infinite;
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 3px solid var(--sf-border);
        border-top-color: var(--sf-accent);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    .spinner-sm {
        width: 12px;
        height: 12px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
        margin-right: 8px;
        display: inline-block;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @keyframes pulse {
        0%,
        100% {
            opacity: 1;
        }
        50% {
            opacity: 0.6;
        }
    }
</style>
