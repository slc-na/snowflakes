<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import NewHostModal from "../components/modal/NewHostModal.svelte";
  import { deleteSessionPass } from "../controller/vault";
  import {
    connectToSession,
    deleteSession,
    reconnectToSession,
  } from "../controller/ssh";
  import type { SessionInfo } from "../types/settings";
  import { deleteSessionInfo, loadAllSessions } from "../controller/local";
  import { clearTerminalState } from "../controller/session";
  import { invoke } from '@tauri-apps/api/core';
  

  let isModalOpen = $state(false);
  let sessions = $state<SessionInfo[]>([]);
  let prefillSession = $state<SessionInfo | null>(null);
  let isLoading = $state(true);

  let isConnecting = $state(false);
  let connectingStatus = $state("");
  let errorMsg = $state("");

  onMount(async () => {
    try {
      sessions = await loadAllSessions();
    } catch (err) {
      console.error("[Home] Failed to load sessions:", err);
    } finally {
      isLoading = false;
    }
  });

  async function handleCardClick(session: SessionInfo) {
    errorMsg = "";
    isConnecting = true;
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
      errorMsg = String(e);
      isConnecting = false;
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
      } catch (err) {
        console.error("[Home] Failed to delete session:", err);
      }
    }
  }

  function handleNewHost() {
    prefillSession = null;
    isModalOpen = true;
  }

  async function handleGetServer() {
    try {
      const serverInfo = await invoke('get_server');
      console.log('Received server info:', serverInfo);
    } catch (err) {
      console.error('Failed to get server info:', err);
      alert('Error fetching server info. Check console for details.');
    }
  }

  async function handleGetShell() {
    try {
      const shellInfo = await invoke('get_shell');
      console.log('Received shell info:', shellInfo);
    } catch (err) {
      console.error('Failed to get shell info:', err);
      alert('Error fetching shell info. Check console for details.');
    }
  }

  function formatDate(ts: number): string {
    return new Date(ts).toLocaleString();
  }
</script>

<main>
  <div class="main-container">
    <div class="host-container">
      <div class="grid">
        <!-- Add New Card -->
        <button class="card card-new" onclick={handleNewHost}>
          <div class="new-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <path d="M10 4v12M4 10h12" />
            </svg>
          </div>
          <span class="new-label">New host</span>
        </button>
        <button class="card card-new" onclick={handleGetServer}>
          <div class="new-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <path d="M10 4v12M4 10h12" />
            </svg>
          </div>
          <span class="new-label">Call Get Server</span>
        </button>
        <button class="card card-new" onclick={handleGetShell}>
          <div class="new-icon">
            <svg
              width="20"
              height="20"
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <path d="M10 4v12M4 10h12" />
            </svg>
          </div>
          <span class="new-label">Call Get Shell</span>
        </button>

        <!-- Host Cards -->
        {#if isLoading}
          {#each Array(5) as _}
            <div class="card skeleton-card">
              <div class="skeleton-top shimmer"></div>
              <div class="skeleton-mid shimmer"></div>
              <div class="skeleton-bot shimmer"></div>
              <div class="skeleton-footer shimmer"></div>
            </div>
          {/each}
        {:else}
          {#each sessions as session}
            <div
              class="card group"
              onclick={() => handleCardClick(session)}
              role="button"
              tabindex="0"
              onkeydown={(e) => e.key === "Enter" && handleCardClick(session)}
            >
              <div class="card-top">
                <div class="card-title-row">
                  <div class="status-dot online"></div>
                  <h3 class="hostname">{session.targetIp}</h3>
                </div>
                <button
                  class="delete-btn"
                  onclick={(e) => handleDeleteClick(e, session.sessionKey)}
                  title="Delete connection"
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                </button>
              </div>

              <div class="card-meta">
                <code class="user-ip"
                  >{session.username}@{session.targetIp}</code
                >
                <span class="label-text">{session.label}</span>
              </div>

              <div class="card-footer">
                <div class="last-seen">
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <circle cx="12" cy="12" r="10" /><path d="M12 6v6l4 2" />
                  </svg>
                  {formatDate(session.connectedAt)}
                </div>
                <span class="connect-hint">
                  Direct Connect
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <path d="M5 12h14M12 5l7 7-7 7" />
                  </svg>
                </span>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>

  {#if isConnecting}
    <div class="global-loading">
      <div class="spinner"></div>
      <span class="loading-msg">{connectingStatus}</span>
      {#if errorMsg}
        <div class="error-toast">
          {errorMsg}
          <button
            onclick={() => {
              errorMsg = "";
              isConnecting = false;
            }}>Dismiss</button
          >
        </div>
      {/if}
    </div>
  {/if}

  <NewHostModal
    isOpen={isModalOpen}
    onClose={() => (isModalOpen = false)}
    prefill={prefillSession}
  />
</main>

<style>
  .main-container {
    width: 100%;
    height: 100vh;
    background-color: var(--sf-bg-app);
    display: flex;
  }

  .host-container {
    flex: 1;
    padding: 32px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 20px;
    width: 100%;
  }

  .card {
    background: var(--sf-bg-surface);
    border-left: 2px solid transparent;
    padding: 24px;
    transition:
      border-color 0.15s,
      background 0.15s;
    position: relative;
    cursor: pointer;
    text-align: left;
  }

  .card:hover {
    border-left-color: var(--sf-accent);
    background: var(--sf-bg-hover);
  }

  .card:hover .connect-hint {
    opacity: 1;
  }

  /* New host card */
  .card-new {
    border: 1px dashed var(--sf-border);
    border-left: 2px solid transparent;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 160px;
  }

  .card-new:hover {
    border-color: var(--sf-accent);
    border-left-color: var(--sf-accent);
    border-style: solid;
  }

  .new-icon {
    color: var(--sf-text-hint);
    transition: color 0.15s;
  }

  .card-new:hover .new-icon {
    color: var(--sf-accent);
  }

  .new-label {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--sf-text-hint);
    transition: color 0.15s;
  }

  .card-new:hover .new-label {
    color: var(--sf-accent);
  }

  /* Card internals */
  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
  }

  .status-dot.online {
    background: var(--sf-online);
  }

  .status-dot.offline {
    background: var(--sf-offline);
  }

  .hostname {
    font-size: 14px;
    font-weight: 600;
    color: var(--sf-text-primary);
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .label-text {
    font-family: var(--sf-font-ui);
    font-size: 11px;
    color: var(--sf-text-secondary);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .delete-btn {
    background: none;
    border: none;
    color: var(--sf-text-hint);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.15s;
    opacity: 0;
  }

  .card:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--sf-status-error);
    background: rgba(239, 83, 80, 0.1);
  }

  .card-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 24px;
  }

  .user-ip {
    font-family: var(--sf-font-mono);
    font-size: 13px;
    color: var(--sf-text-primary);
    opacity: 0.9;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .last-seen {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--sf-text-hint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }

  .last-seen svg {
    color: var(--sf-text-hint);
    opacity: 0.7;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(26, 51, 82, 0.5);
    padding-top: 12px;
  }

  .connect-hint {
    opacity: 0;
    transition: opacity 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--sf-accent);
  }

  /* ── Loading Overlay ────────────────────────────────── */
  .global-loading {
    position: fixed;
    inset: 0;
    background: rgba(5, 15, 28, 0.9);
    backdrop-filter: blur(8px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 100;
    gap: 16px;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--sf-border);
    border-top-color: var(--sf-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .loading-msg {
    color: var(--sf-text-primary);
    font-size: 14px;
    font-weight: 500;
    letter-spacing: 0.05em;
  }

  .error-toast {
    margin-top: 24px;
    background: var(--sf-status-error);
    color: white;
    padding: 12px 18px;
    border-radius: var(--sf-radius-md);
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12px;
    animation: slide-up 0.3s ease;
  }

  .error-toast button {
    background: rgba(255, 255, 255, 0.2);
    border: none;
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
  }

  /* ── Skeleton UI ────────────────────────────────────── */
  .skeleton-card {
    pointer-events: none;
    min-height: 160px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .skeleton-top {
    height: 20px;
    width: 60%;
    background: var(--sf-bg-app);
    border-radius: 4px;
  }

  .skeleton-mid {
    height: 14px;
    width: 80%;
    background: var(--sf-bg-app);
    border-radius: 4px;
  }

  .skeleton-bot {
    height: 12px;
    width: 40%;
    background: var(--sf-bg-app);
    border-radius: 4px;
    margin-bottom: auto;
  }

  .skeleton-footer {
    height: 30px;
    width: 100%;
    background: var(--sf-bg-app);
    border-top: 1px solid rgba(255, 255, 255, 0.03);
    border-radius: 0 0 4px 4px;
  }

  .shimmer {
    position: relative;
    overflow: hidden;
  }

  .shimmer::after {
    content: "";
    position: absolute;
    inset: 0;
    transform: translateX(-100%);
    background: linear-gradient(
      90deg,
      transparent,
      rgba(255, 255, 255, 0.05),
      transparent
    );
    animation: shimmer 1.5s infinite;
  }

  @keyframes shimmer {
    100% {
      transform: translateX(100%);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes slide-up {
    from {
      transform: translateY(10px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }
</style>
