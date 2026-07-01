<script lang="ts">
    import type { ServerAttribute, ServerOs } from "../../types/servers";

    let {
        isOpen,
        server,
        onClose,
        onSave,
    } = $props<{
        isOpen: boolean;
        server: ServerAttribute | null;
        onClose: () => void;
        onSave: (server: ServerAttribute) => Promise<void>;
    }>();

    let name = $state("");
    let ip = $state("");
    let port = $state(22);
    let os = $state<ServerOs>("linux");
    let description = $state("");
    let isSaving = $state(false);
    let errorMsg = $state("");

    $effect(() => {
        if (!isOpen || !server) return;
        name = server.name;
        ip = server.ip;
        port = server.ssh_port;
        os = server.os;
        description = server.description;
        errorMsg = "";
    });

    async function handleSave() {
        if (!server) return;
        errorMsg = "";
        isSaving = true;
        try {
            await onSave({ ...server, name, ip, ssh_port: port, os, description });
            onClose();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        } finally {
            isSaving = false;
        }
    }
</script>

{#if isOpen && server}
    <div class="overlay" role="dialog">
        <div class="modal">
            <div class="modal-header">
                <span class="modal-title">
                    Edit SSH & SFTP connection
                </span>
                <button class="close-btn" onclick={onClose}>✕</button>
            </div>

            <div class="modal-body">
                <div class="warning-banner">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                        <line x1="12" y1="9" x2="12" y2="13" />
                        <line x1="12" y1="17" x2="12.01" y2="17" />
                    </svg>
                    Changes will be applied to the database and affect every user of this server.
                </div>

                <div class="field">
                    <label for="edit-name">Name</label>
                    <input id="edit-name" type="text" bind:value={name} />
                </div>

                <div class="row">
                    <div class="field" style="flex:1">
                        <label for="edit-ip">Hostname / IP</label>
                        <input id="edit-ip" type="text" bind:value={ip} />
                    </div>
                    <div class="field" style="width:90px">
                        <label for="edit-port">Port</label>
                        <input id="edit-port" type="number" bind:value={port} />
                    </div>
                </div>

                <div class="field">
                    <label for="edit-os">Operating System</label>
                    <select id="edit-os" bind:value={os}>
                        <option value="linux">Linux</option>
                        <option value="mac">macOS</option>
                        <option value="windows">Windows</option>
                    </select>
                </div>

                <div class="field">
                    <label for="edit-description">Description</label>
                    <input id="edit-description" type="text" bind:value={description} />
                </div>

                {#if errorMsg}
                    <div class="error-msg">{errorMsg}</div>
                {/if}
            </div>

            <div class="modal-footer">
                <button class="btn-ghost" onclick={onClose} disabled={isSaving}>Cancel</button>
                <button class="btn-primary" onclick={handleSave} disabled={isSaving}>
                    {isSaving ? "Saving…" : "Save changes"}
                </button>
            </div>
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
        z-index: 60;
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

    .warning-banner {
        display: flex;
        align-items: center;
        gap: 8px;
        background: rgba(255, 179, 71, 0.1);
        border: 1px solid rgba(255, 179, 71, 0.3);
        color: #ffb347;
        border-radius: 7px;
        padding: 8px 10px;
        font-size: 11px;
        line-height: 1.4;
    }

    .warning-banner svg {
        flex-shrink: 0;
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

    input,
    select {
        background: var(--sf-bg-input);
        border: 1px solid var(--sf-border);
        border-radius: 7px;
        padding: 8px 10px;
        font-size: 12px;
        color: var(--sf-text-primary);
        outline: none;
        width: 100%;
        font-family: var(--sf-font-ui);
        box-sizing: border-box;
        cursor: pointer;
        appearance: none;
        -webkit-appearance: none;
        -moz-appearance: none;
        background-repeat: no-repeat;
        background-position: right 10px center;
        padding-right: 28px;
    }
    
    select{
        background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' fill='none'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%23888' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    }

    input:focus,
    select:focus {
        border-color: var(--sf-accent);
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

    .btn-primary:disabled,
    .btn-ghost:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
