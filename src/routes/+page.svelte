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
  import { deleteSessionInfo, getAllServers, loadAllSessions } from "../controller/local";
  import { clearTerminalState } from "../controller/session";
  import { invoke } from '@tauri-apps/api/core';
  import type { ServerAttribute } from "../types/servers";

  let isModalOpen = $state(false);
  let sessions = $state<SessionInfo[]>([]);
  let servers = $state<ServerAttribute[]>([]);
  let prefillSession = $state<SessionInfo | null>(null);
  let isLoading = $state(true);

  let isConnecting = $state(false);
  let connectingStatus = $state("");
  let errorMsg = $state("");

  let searchQuery = $state("");
  let sortKey = $state<"name" | "ip">("name");
  let sortDir = $state<"asc" | "desc">("asc");
  let filterOnline = $state(false); 

  let filteredServers = $derived(() => {
    let list = [...servers];
    console.log(list);

    const q = searchQuery.trim().toLowerCase();
    if (q) {
      list = list.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.ip.toLowerCase().includes(q) ||
          s.description.toLowerCase().includes(q),
      );
    }

    list.sort((a, b) => {
      const av = sortKey === "ip" ? a.ip : a.name.toLowerCase();
      const bv = sortKey === "ip" ? b.ip : b.name.toLowerCase();
      if (av < bv) return sortDir === "asc" ? -1 : 1;
      if (av > bv) return sortDir === "asc" ? 1 : -1;
      return 0;
    });

    return list;
  });

  onMount(async () => {
    try {
      sessions = await loadAllSessions();
      servers = await getAllServers();
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

  function handleServerCardClick(server: ServerAttribute) {
    prefillSession = {
      sessionKey: "",          
      targetIp: server.ip,
      label: server.name,
      username: "",
      password: "",
      bastionIp: "",
      connectedAt: 0,
    };
    isModalOpen = true;
  }

  function toggleSort(key: "name" | "ip") {
    if (sortKey === key) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
    } else {
      sortKey = key;
      sortDir = "asc";
    }
  }

  function formatDate(ts: number): string {
    return new Date(ts).toLocaleString();
  }
</script>

<main>
  <div class="main-container">
    <div class="page-content">

      <!-- ══════════════════════ SERVERS SECTION ══════════════════════ -->
      <section class="section">
        <div class="section-header">
          <div class="section-title-group">
            <span class="section-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect>
                <rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect>
                <line x1="6" y1="6" x2="6.01" y2="6"></line>
                <line x1="6" y1="18" x2="6.01" y2="18"></line>
              </svg>
            </span>
            <h2 class="section-title">Servers</h2>
            <span class="section-count">{filteredServers().length}</span>
          </div>

          <!-- Toolbar: Search + Sort -->
          <div class="toolbar">
            <div class="search-wrap">
              <svg class="search-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/>
              </svg>
              <input
                id="server-search"
                class="search-input"
                type="text"
                placeholder="Search servers…"
                bind:value={searchQuery}
              />
              {#if searchQuery}
                <button class="search-clear" onclick={() => (searchQuery = "")}>✕</button>
              {/if}
            </div>

            <div class="sort-group">
              <span class="sort-label">Sort</span>
              <button
                id="sort-by-name"
                class="sort-btn"
                class:active={sortKey === "name"}
                onclick={() => toggleSort("name")}
              >
                Name
                {#if sortKey === "name"}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                    {#if sortDir === "asc"}
                      <path d="M12 19V5M5 12l7-7 7 7"/>
                    {:else}
                      <path d="M12 5v14M5 12l7 7 7-7"/>
                    {/if}
                  </svg>
                {/if}
              </button>
              <button
                id="sort-by-ip"
                class="sort-btn"
                class:active={sortKey === "ip"}
                onclick={() => toggleSort("ip")}
              >
                IP
                {#if sortKey === "ip"}
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                    {#if sortDir === "asc"}
                      <path d="M12 19V5M5 12l7-7 7 7"/>
                    {:else}
                      <path d="M12 5v14M5 12l7 7 7-7"/>
                    {/if}
                  </svg>
                {/if}
              </button>
            </div>
          </div>
        </div>

        <!-- Server Grid -->
        {#if isLoading}
          <div class="grid">
            {#each Array(4) as _}
              <div class="card skeleton-card">
                <div class="skeleton-top shimmer"></div>
                <div class="skeleton-mid shimmer"></div>
                <div class="skeleton-bot shimmer"></div>
                <div class="skeleton-footer shimmer"></div>
              </div>
            {/each}
          </div>
        {:else if filteredServers().length === 0}
          <div class="empty-state">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" opacity="0.3">
              <rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect>
              <rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect>
              <line x1="6" y1="6" x2="6.01" y2="6"></line>
              <line x1="6" y1="18" x2="6.01" y2="18"></line>
            </svg>
            <span class="empty-title">
              {searchQuery ? "No servers match your search" : "No servers found"}
            </span>
            <span class="empty-sub">
              {searchQuery ? "Try a different keyword or clear the filter" : "Servers will appear here once loaded"}
            </span>
          </div>
        {:else}
          <div class="grid">
            {#each filteredServers() as server (server.id)}
              <button
                class="card server-card"
                id="server-{server.id}"
                onclick={() => handleServerCardClick(server)}
              >
                <div class="card-top">
                  <div class="card-title-row">
                    <div class="server-badge">
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <rect x="2" y="2" width="20" height="8" rx="2"></rect>
                        <line x1="6" y1="6" x2="6.01" y2="6"></line>
                      </svg>
                    </div>
                    <h3 class="hostname">{server.name}</h3>
                  </div>
                  <span class="connect-hint">
                    Connect
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <path d="M5 12h14M12 5l7 7-7 7"/>
                    </svg>
                  </span>
                </div>

                <div class="card-meta">
                  <code class="user-ip">{server.ip}</code>
                  {#if server.description}
                    <span class="label-text">{server.description}</span>
                  {/if}
                </div>

                <div class="card-footer">
                  <span class="ip-badge">
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"></path>
                      <circle cx="12" cy="10" r="3"></circle>
                    </svg>
                    {server.ip}
                  </span>
                  <span class="click-hint">Click to connect</span>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <!-- ═══════════════════ RECENT SESSIONS SECTION ═════════════════ -->
      <section class="section">
        <div class="section-header">
          <div class="section-title-group">
            <span class="section-icon">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>
              </svg>
            </span>
            <h2 class="section-title">Recent Sessions</h2>
            <span class="section-count">{sessions.length}</span>
          </div>

          <button class="btn-new-host" id="new-host-btn" onclick={handleNewHost}>
            <svg width="13" height="13" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10 4v12M4 10h12"/>
            </svg>
            New Host
          </button>
        </div>

        {#if isLoading}
          <div class="grid">
            {#each Array(3) as _}
              <div class="card skeleton-card">
                <div class="skeleton-top shimmer"></div>
                <div class="skeleton-mid shimmer"></div>
                <div class="skeleton-bot shimmer"></div>
                <div class="skeleton-footer shimmer"></div>
              </div>
            {/each}
          </div>
        {:else if sessions.length === 0}
          <div class="empty-state">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" opacity="0.3">
              <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path>
              <polyline points="13 2 13 9 20 9"></polyline>
            </svg>
            <span class="empty-title">No recent sessions</span>
            <span class="empty-sub">Click <strong>New Host</strong> or pick a server above to start a connection</span>
          </div>
        {:else}
          <div class="grid">
            {#each sessions as session (session.sessionKey)}
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
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <line x1="18" y1="6" x2="6" y2="18"></line>
                      <line x1="6" y1="6" x2="18" y2="18"></line>
                    </svg>
                  </button>
                </div>

                <div class="card-meta">
                  <code class="user-ip">{session.username}@{session.targetIp}</code>
                  <span class="label-text">{session.label}</span>
                </div>

                <div class="card-footer">
                  <div class="last-seen">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>
                    </svg>
                    {formatDate(session.connectedAt)}
                  </div>
                  <span class="connect-hint">
                    Direct Connect
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M5 12h14M12 5l7 7-7 7"/>
                    </svg>
                  </span> 
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </section>

    </div>
  </div>

  <!-- ── Global connecting overlay ─────────────────────────── -->
  {#if isConnecting}
    <div class="global-loading">
      <div class="spinner"></div>
      <span class="loading-msg">{connectingStatus}</span>
      {#if errorMsg}
        <div class="error-toast">
          {errorMsg}
          <button onclick={() => { errorMsg = ""; isConnecting = false; }}>Dismiss</button>
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
  /* ── Layout ──────────────────────────────────────────────── */
  .main-container {
    width: 100%;
    height: 100vh;
    background-color: var(--sf-bg-app);
    overflow-y: auto;
    display: flex;
  }

  .page-content {
    flex: 1;
    padding: 32px 36px;
    display: flex;
    flex-direction: column;
    gap: 40px;
    max-width: 1400px;
  }

  /* ── Section ──────────────────────────────────────────────── */
  .section {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
  }

  .section-title-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-icon {
    color: var(--sf-accent);
    display: flex;
    align-items: center;
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--sf-text-primary);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    margin: 0;
  }

  .section-count {
    font-size: 10px;
    font-weight: 600;
    color: var(--sf-accent);
    background: var(--sf-accent-dim);
    border: 1px solid rgba(79, 195, 247, 0.2);
    border-radius: var(--sf-radius-pill);
    padding: 1px 7px;
    letter-spacing: 0.05em;
  }

  /* ── Toolbar ──────────────────────────────────────────────── */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 9px;
    color: var(--sf-text-hint);
    pointer-events: none;
  }

  .search-input {
    background: var(--sf-bg-surface);
    border: 1px solid var(--sf-border);
    border-radius: var(--sf-radius-md);
    padding: 6px 28px 6px 28px;
    font-size: 12px;
    color: var(--sf-text-primary);
    outline: none;
    width: 220px;
    font-family: var(--sf-font-ui);
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .search-input:focus {
    border-color: var(--sf-accent);
    box-shadow: 0 0 0 2px var(--sf-accent-glow);
  }

  .search-input::placeholder {
    color: var(--sf-text-hint);
  }

  .search-clear {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--sf-text-hint);
    cursor: pointer;
    font-size: 10px;
    padding: 2px;
    line-height: 1;
    transition: color 0.15s;
  }

  .search-clear:hover {
    color: var(--sf-text-primary);
  }

  .sort-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .sort-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--sf-text-hint);
    margin-right: 2px;
  }

  .sort-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--sf-bg-surface);
    border: 1px solid var(--sf-border);
    border-radius: var(--sf-radius-sm);
    padding: 5px 10px;
    font-size: 11px;
    font-weight: 500;
    color: var(--sf-text-secondary);
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--sf-font-ui);
  }

  .sort-btn:hover {
    border-color: var(--sf-border-hover);
    color: var(--sf-text-primary);
  }

  .sort-btn.active {
    border-color: var(--sf-accent);
    color: var(--sf-accent);
    background: var(--sf-accent-dim);
  }

  /* ── New Host Button ──────────────────────────────────────── */
  .btn-new-host {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--sf-accent);
    border: none;
    border-radius: var(--sf-radius-md);
    padding: 7px 14px;
    font-size: 12px;
    font-weight: 600;
    color: var(--sf-text-on-accent);
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s;
    font-family: var(--sf-font-ui);
    letter-spacing: 0.02em;
  }

  .btn-new-host:hover {
    background: var(--sf-accent-hover);
    box-shadow: 0 0 12px var(--sf-accent-glow);
  }

  /* ── Grid ──────────────────────────────────────────────────── */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    width: 100%;
  }

  /* ── Cards (shared) ───────────────────────────────────────── */
  .card {
    background: var(--sf-bg-surface);
    border: 1px solid var(--sf-border);
    border-left: 2px solid transparent;
    padding: 20px;
    transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
    position: relative;
    cursor: pointer;
    text-align: left;
    border-radius: var(--sf-radius-md);
  }

  .card:hover {
    border-left-color: var(--sf-accent);
    background: var(--sf-bg-hover);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
  }

  /* ── Server Card ──────────────────────────────────────────── */
  .server-card {
    border-left: 2px solid transparent;
  }

  .server-card:hover .connect-hint {
    opacity: 1;
  }

  .server-badge {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--sf-accent-dim);
    border: 1px solid rgba(79, 195, 247, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--sf-accent);
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .server-card:hover .server-badge {
    background: rgba(79, 195, 247, 0.2);
  }

  .ip-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: var(--sf-font-mono);
    font-size: 10px;
    color: var(--sf-text-hint);
    letter-spacing: 0.02em;
  }

  .click-hint {
    font-size: 10px;
    color: var(--sf-text-hint);
    opacity: 0;
    transition: opacity 0.15s;
    letter-spacing: 0.03em;
  }

  .server-card:hover .click-hint {
    opacity: 1;
  }

  /* ── Card internals ───────────────────────────────────────── */
  .card-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .card-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-dot.online {
    background: var(--sf-status-online);
    box-shadow: 0 0 6px rgba(38, 198, 160, 0.5);
  }

  .status-dot.offline {
    background: var(--sf-status-offline);
  }

  .hostname {
    font-size: 13px;
    font-weight: 600;
    color: var(--sf-text-primary);
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
    min-width: 0;
    flex: 1;
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
    flex-shrink: 0;
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
    margin-bottom: 16px;
  }

  .user-ip {
    font-family: var(--sf-font-mono);
    font-size: 12px;
    color: var(--sf-text-primary);
    opacity: 0.85;
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

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(26, 51, 82, 0.6);
    padding-top: 12px;
  }

  .connect-hint {
    opacity: 0;
    transition: opacity 0.15s;
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--sf-accent);
    flex-shrink: 0;
  }

  /* ── Empty state ──────────────────────────────────────────── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 48px 24px;
    border: 1px dashed var(--sf-border);
    border-radius: var(--sf-radius-lg);
    background: var(--sf-bg-surface);
  }

  .empty-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--sf-text-secondary);
    margin-top: 8px;
  }

  .empty-sub {
    font-size: 11px;
    color: var(--sf-text-hint);
    text-align: center;
    max-width: 260px;
    line-height: 1.6;
  }

  .empty-sub strong {
    color: var(--sf-accent);
  }

  /* ── Loading Overlay ──────────────────────────────────────── */
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

  /* ── Skeleton UI ──────────────────────────────────────────── */
  .skeleton-card {
    pointer-events: none;
    min-height: 148px;
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
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.05), transparent);
    animation: shimmer 1.5s infinite;
  }

  @keyframes shimmer {
    100% { transform: translateX(100%); }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes slide-up {
    from { transform: translateY(10px); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }
</style>
