<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from '@tauri-apps/api/core';
    import {
        saveDefaultAccount,
        loadDefaultAccount,
        deleteSessionPass,
    } from "../../controller/vault";
    import type {
        SnowflakesSettings,
        DefaultAccount,
        SessionInfo,
    } from "../../types/settings";
    import { DEFAULT_SETTINGS } from "../../types/settings";
    import {
        deleteSessionInfo,
        loadAllSessions,
        loadSettings,
        saveSettings,
    } from "../../controller/local";

    let bastionIp = $state("");
    let sshTemplate = $state(DEFAULT_SETTINGS.sshTemplate);
    let apiKey = $state("");
    let backendUrl = $state("");
    let fontSize = $state(13);

    let fontSizeOptions = $state(Array(40).keys().map(size => size + 6))

    let accountUsername = $state("");
    let accountPassword = $state("");
    let showAccountPassword = $state(false);

    let sessions = $state<SessionInfo[]>([]);
    let hostsOpen = $state(false);

    let statusMessage = $state("");

    function showStatus(msg: string) {
        statusMessage = msg;
    }

    // SSH template preview
    let templatePreview = $derived(
        sshTemplate
            .replace(/\{username\}/g, accountUsername || "user")
            .replace(/\{bastion\}/g, bastionIp || "bastion")
            .replace(/\{target\}/g, "192.168.1.1"),
    );

    onMount(async () => {
        try {
            const settings = await loadSettings();
            bastionIp = settings.bastionIp;
            sshTemplate = settings.sshTemplate;
            apiKey = settings.apiKey;
            backendUrl = settings.backendUrl;
            fontSize = settings.fontSize;
            
            
            const account = await loadDefaultAccount();
            accountUsername = account.username;
            accountPassword = account.password;
         
            // getting username based on oauth
            if(!accountUsername){
                const oauthUser: any = await invoke("get_user_info");
                if (oauthUser && oauthUser.username) {
                    accountUsername = oauthUser.username + "@ad.slc.net";
                }

                const account: DefaultAccount = {
                    username: accountUsername,
                    password: accountPassword,
                };
                await saveDefaultAccount(account);
            }

            // get bastion IP from backend if not set 
            if(!bastionIp){
                try {
                    const bastionIpResult: any = await invoke("get_bastion_ip");
                    if (bastionIpResult) {
                        bastionIp = bastionIpResult;

                        // Save the fetched bastion IP to settings
                        const settings: SnowflakesSettings = {
                            bastionIp,
                            sshTemplate,
                            apiKey,
                            backendUrl,
                            fontSize
                        };
                        await saveSettings(settings);
                    }
                } catch (err) {
                    console.error("Failed to fetch backend config:", err);
                }
            }


            if(!accountUsername.endsWith("@ad.slc.net")){
                const account: DefaultAccount = {
                    username: accountUsername + "@ad.slc.net",
                    password: accountPassword,
                };
                await saveDefaultAccount(account);
            }

            sessions = await loadAllSessions();

        } catch (err) {
            console.error("[Settings] load error:", err);
        }
    });

    async function handleSaveSettings() {
        try {
            const settings: SnowflakesSettings = {
                bastionIp,
                sshTemplate,
                apiKey,
                backendUrl,
                fontSize,
            };
            saveSettings(settings);

            const account: DefaultAccount = {
                username: accountUsername,
                password: accountPassword,
            };
            await saveDefaultAccount(account);

            showStatus("Settings saved successfully");
        } catch (err) {
            console.error("[Settings] save error:", err);
            showStatus("Failed to save settings");
        }
    }

    async function handleDeleteSession(key: string) {
        try {
            await deleteSessionInfo(key);
            await deleteSessionPass(key);
            sessions = sessions.filter((s) => s.sessionKey !== key);
            showStatus("Host removed");
        } catch (err) {
            console.error("[Settings] delete session error:", err);
        }
    }

    function formatDate(ts: number): string {
        return new Date(ts).toLocaleString();
    }
</script>

<div class="settings-page">
    <div class="settings-header">
        <h1 class="page-title">Settings</h1>
        <p class="page-subtitle">Configure Snowflakes SSH Manager</p>
    </div>
    <div class="settings-grid">
        <div class="settings-column">
            <!-- Connection Defaults -->
            <section class="section">
                <h2 class="section-title">Connection Defaults</h2>
                <div class="section-body">
                    <div class="field">
                        <label for="bastion-ip">Default Bastion Server IP</label
                        >
                        <input
                            id="bastion-ip"
                            type="text"
                            placeholder="10.22.77.251"
                            bind:value={bastionIp}
                        />
                    </div>

                    <div class="field">
                        <label for="ssh-template">SSH Command Template</label>
                        <input
                            id="ssh-template"
                            type="text"
                            placeholder="ssh -t [username]@[bastion] [target]"
                            bind:value={sshTemplate}
                        />
                        <span class="field-hint"
                            >Available variables: <code>{"{username}"}</code>,
                            <code>{"{bastion}"}</code>,
                            <code>{"{target}"}</code></span
                        >
                    </div>

                    <div class="field">
                        <label>Preview</label>
                        <div class="cmd-box">
                            <span class="cmd-text">{templatePreview}</span>
                        </div>
                    </div>
                </div>
            </section>
        </div>

        <div class="settings-column">
            <!-- API Configuration -->
            <section class="section">
                <h2 class="section-title">API Configuration</h2>
                <div class="section-body">
                    <div class="field">
                        <label for="api-key">SNOWFLAKES_API_KEY</label>
                        <input
                            id="api-key"
                            type="password"
                            placeholder="Enter API key"
                            bind:value={apiKey}
                        />
                    </div>

                    <div class="field">
                        <label for="backend-url">SNOWFLAKES_BACKEND_URL</label>
                        <input
                            id="backend-url"
                            type="text"
                            placeholder="https://api.snowflakes.example.com"
                            bind:value={backendUrl}
                        />
                    </div>
                </div>
            </section>
        </div>

        <div class="settings-column">
            <!-- Default Account -->
            <section class="section">
                <h2 class="section-title">Default Account</h2>
                <div class="section-body">
                    <div class="field">
                        <label for="acct-username">Username</label>
                        <input
                            id="acct-username"
                            type="text"
                            placeholder="root"
                            bind:value={accountUsername}
                        />
                    </div>

                    <div class="field">
                        <label for="acct-password">Password</label>
                        <div class="input-wrap">
                            <input
                                id="acct-password"
                                type={showAccountPassword ? "text" : "password"}
                                placeholder="••••••••"
                                bind:value={accountPassword}
                            />
                            <button
                                class="eye-btn"
                                onclick={() =>
                                    (showAccountPassword =
                                        !showAccountPassword)}
                            >
                                {showAccountPassword ? "hide" : "show"}
                            </button>
                        </div>
                    </div>
                </div>
            </section>
        </div>
    </div>

    <div class="settings-footer">
        <!-- Known Hosts Accordion -->
        <section class="section">
            <button
                class="accordion-toggle"
                onclick={() => (hostsOpen = !hostsOpen)}
                aria-expanded={hostsOpen}
            >
                <h2 class="section-title" style="margin:0">
                    Known Hosts ({sessions.length})
                </h2>
                <span class="accordion-arrow" class:open={hostsOpen}>
                    <svg
                        width="12"
                        height="12"
                        viewBox="0 0 12 12"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    >
                        <polyline points="3,4 6,7 9,4" />
                    </svg>
                </span>
            </button>

            {#if hostsOpen}
                <div class="accordion-body">
                    {#if sessions.length === 0}
                        <p class="empty-hint">
                            No previously connected hosts found.
                        </p>
                    {:else}
                        {#each sessions as session}
                            <div class="host-row">
                                <div class="host-info">
                                    <span class="host-label"
                                        >{session.label}</span
                                    >
                                    <code class="host-detail"
                                        >{session.username}@{session.targetIp}
                                        via {session.bastionIp}</code
                                    >
                                    <span class="host-time"
                                        >{formatDate(session.connectedAt)}</span
                                    >
                                </div>
                                <button
                                    class="delete-btn"
                                    title="Remove host"
                                    onclick={() =>
                                        handleDeleteSession(session.sessionKey)}
                                >
                                    <svg
                                        width="10"
                                        height="10"
                                        viewBox="0 0 10 10"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="1.5"
                                        stroke-linecap="round"
                                    >
                                        <line x1="2" y1="2" x2="8" y2="8" />
                                        <line x1="8" y1="2" x2="2" y2="8" />
                                    </svg>
                                </button>
                            </div>
                        {/each}
                    {/if}
                </div>
            {/if}
        </section>


        <!-- font size selection -->
        <section class="section">
            <h2 class="section-title">Preferred Font Size</h2>
            <div class="section-body">
                <div class="field">
                    <label for="acct-username">Font Size</label>

                    <select bind:value={fontSize}>
                        {#each fontSizeOptions as num}
                            <option value={num}>
                                {num}px
                            </option>
                        {/each}
                    </select>
                </div>
            </div>
        </section>

        <!-- Save -->
        <div class="save-bar">
            {#if statusMessage}
                <span class="status-msg">{statusMessage}</span>
            {/if}
            <button class="btn-primary" onclick={handleSaveSettings}
                >Save settings</button
            >
        </div>
    </div>
</div>

<style>
    .settings-grid {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 24px;
        padding: 24px 40px;
    }

    .settings-column {
        display: flex;
        flex-direction: column;
        gap: 24px;
    }

    .settings-column > .section {
        height: 100%;
    }

    .settings-footer {
        padding: 0 40px 40px;
        display: flex;
        flex-direction: column;
        gap: 24px;
    }

    .settings-page {
        width: 100%;
        height: 100%;
        background-color: var(--sf-bg-app);
        overflow-y: auto;
    }

    .settings-header {
        padding: 32px 40px 0;
    }

    .page-title {
        font-family: var(--sf-font-title);
        font-size: 1.25rem;
        font-weight: 800;
        color: var(--sf-text-primary);
        margin: 0;
    }

    .page-subtitle {
        font-family: var(--sf-font-ui);
        font-size: 12px;
        color: var(--sf-text-secondary);
        margin: 6px 0 0 0;
    }

    /* ── Sections ─────────────────────────────────────── */
    .section {
        background: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-lg);
        overflow: hidden;
    }

    .section-title {
        font-family: var(--sf-font-ui);
        font-size: 12px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.08em;
        color: var(--sf-text-secondary);
        margin: 0;
    }

    .section-body {
        padding: 16px 18px;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .section-body:not(:first-child) {
        border-top: none;
    }

    .section > .section-title {
        padding: 14px 18px 0;
    }

    /* ── Fields ───────────────────────────────────────── */
    .field {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    label {
        font-family: var(--sf-font-ui);
        font-size: 11px;
        color: var(--sf-text-primary);
    }

    input {
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
    }

    input:focus {
        border-color: var(--sf-accent);
    }

    input::placeholder {
        color: var(--sf-text-hint);
    }

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
        background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' fill='none'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%23888' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
        background-repeat: no-repeat;
        background-position: right 10px center;
        padding-right: 28px;
    }

    select:focus {
        border-color: var(--sf-accent);
    }

    .field-hint {
        font-size: 10px;
        color: var(--sf-text-hint);
        font-family: var(--sf-font-ui);
    }

    .field-hint code {
        font-family: var(--sf-font-mono);
        color: var(--sf-text-accent);
        font-size: 10px;
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
        font-family: var(--sf-font-ui);
    }

    .eye-btn:hover {
        color: var(--sf-text-secondary);
    }

    /* ── Command preview ──────────────────────────────── */
    .cmd-box {
        background: var(--sf-bg-terminal);
        border: 1px solid var(--sf-border);
        border-radius: 7px;
        padding: 10px 12px;
    }

    .cmd-text {
        font-family: var(--sf-font-mono);
        font-size: 11px;
        color: var(--sf-accent);
        word-break: break-all;
    }

    /* ── Accordion ────────────────────────────────────── */
    .accordion-toggle {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 14px 18px;
        background: none;
        border: none;
        cursor: pointer;
        color: var(--sf-text-secondary);
    }

    .accordion-toggle:hover {
        background: var(--sf-bg-hover);
    }

    .accordion-arrow {
        transition: transform 0.2s ease;
        display: flex;
    }

    .accordion-arrow.open {
        transform: rotate(180deg);
    }

    .accordion-body {
        border-top: 1px solid var(--sf-border);
        padding: 8px 18px 14px;
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .empty-hint {
        font-size: 11px;
        color: var(--sf-text-hint);
        font-style: italic;
        margin: 8px 0;
        font-family: var(--sf-font-ui);
    }

    .host-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 10px;
        border-radius: var(--sf-radius-sm);
        transition: background 0.12s;
    }

    .host-row:hover {
        background: var(--sf-bg-hover);
    }

    .host-info {
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
    }

    .host-label {
        font-family: var(--sf-font-ui);
        font-size: 12px;
        font-weight: 500;
        color: var(--sf-text-primary);
    }

    .host-detail {
        font-family: var(--sf-font-mono);
        font-size: 10px;
        color: var(--sf-text-accent);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .host-time {
        font-family: var(--sf-font-ui);
        font-size: 9px;
        color: var(--sf-text-hint);
        text-transform: uppercase;
        letter-spacing: 0.04em;
    }

    .delete-btn {
        flex-shrink: 0;
        width: 22px;
        height: 22px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: none;
        border: 1px solid transparent;
        border-radius: var(--sf-radius-sm);
        color: var(--sf-text-hint);
        cursor: pointer;
        transition:
            background 0.12s,
            color 0.12s,
            border-color 0.12s;
    }

    .delete-btn:hover {
        background: rgba(239, 83, 80, 0.12);
        color: var(--sf-status-error);
        border-color: rgba(239, 83, 80, 0.25);
    }

    /* ── Save bar ─────────────────────────────────────── */
    .save-bar {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 12px;
    }

    .status-msg {
        font-family: var(--sf-font-ui);
        font-size: 11px;
        color: var(--sf-status-online);
        animation: fade-in 0.15s ease;
    }

    @keyframes fade-in {
        from {
            opacity: 0;
        }
        to {
            opacity: 1;
        }
    }

    .btn-primary {
        background: var(--sf-accent);
        border: none;
        border-radius: 7px;
        padding: 8px 18px;
        font-size: 12px;
        font-weight: 500;
        color: var(--sf-text-on-accent);
        cursor: pointer;
        font-family: var(--sf-font-ui);
        transition: background 0.12s;
    }

    .btn-primary:hover {
        background: var(--sf-accent-hover);
    }
</style>
