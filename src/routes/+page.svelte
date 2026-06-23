<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toast } from "svelte-sonner";
  import NewHostModal from "../components/modal/NewHostModal.svelte";
  import EditServerModal from "../components/modal/EditServerModal.svelte";
  import type { SessionInfo } from "../types/settings";
  import { getAllServers, getCachedServers, updateServer, loadSettings } from "../controller/local";
  import { loadDefaultAccount } from "../controller/vault";
  import { connectToSession } from "../controller/ssh";
  import * as sftp from "../controller/sftp";
  import { connectToGuacamoleSession } from "../controller/guacamole";
  import type { ServerAttribute } from "../types/servers";

  let isModalOpen = $state(false);
  let servers = $state<ServerAttribute[]>([]);
  let prefillSession = $state<SessionInfo | null>(null);
  let isLoading = $state(true);

  let isConnecting = $state(false);
  let connectingStatus = $state("");
  let errorMsg = $state("");

  let editModalOpen = $state(false);
  let editModalServer = $state<ServerAttribute | null>(null);

  let searchQuery = $state("");
  let sortKey = $state<"name" | "ip">("name");
  let sortDir = $state<"asc" | "desc">("asc");
  let searchInputEl = $state<HTMLInputElement | null>(null);

  let filteredServers = $derived(() => {
    let list = [...servers];

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
    searchInputEl?.focus();

    const cached = getCachedServers();
    if (cached) {
      servers = cached;
      isLoading = false;
    }

    try {
      servers = await getAllServers();
    } catch (err) {
      console.error("[Home] Failed to load servers:", err);
      if (!cached) toast.error("Failed to load servers.");
    } finally {
      isLoading = false;
    }
  });

  function handleNewHost() {
    prefillSession = null;
    isModalOpen = true;
  }

  async function getQuickConnectDefaults() {
    const account = await loadDefaultAccount();
    if (!account.username) {
      toast.error("Set a Default Account in Settings before using Quick Connect.");
      return null;
    }

    const settings = await loadSettings();
    if (!settings.bastionIp) {
      toast.error("Set a Default Bastion Server IP in Settings before using Quick Connect.");
      return null;
    }

    return { account, bastionIp: settings.bastionIp };
  }

  async function handleQuickSsh(e: MouseEvent, server: ServerAttribute) {
    e.stopPropagation();
    const defaults = await getQuickConnectDefaults();
    if (!defaults) return;

    isConnecting = true;
    connectingStatus = "Connecting…";
    errorMsg = "";
    try {
      await connectToSession(
        {
          sessionKey: "",
          username: defaults.account.username,
          password: defaults.account.password,
          targetIp: server.ip,
          port: server.ssh_port,
          bastionIp: defaults.bastionIp,
          label: server.name,
          connectedAt: Date.now(),
        },
        (msg) => {
          connectingStatus = msg;
        },
      );
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      isConnecting = false;
    }
  }

  async function handleQuickSftp(e: MouseEvent, server: ServerAttribute) {
    e.stopPropagation();
    const defaults = await getQuickConnectDefaults();
    if (!defaults) return;

    isConnecting = true;
    connectingStatus = "Establishing SFTP tunnel…";
    try {
      const key = await sftp.connectToSftpSession(
        {
          sessionKey: "",
          username: defaults.account.username,
          password: defaults.account.password,
          targetIp: server.ip,
          bastionIp: defaults.bastionIp,
          label: server.name,
          connectedAt: Date.now(),
        },
        (msg) => {
          connectingStatus = msg;
        },
      );
      goto(`/files?key=${encodeURIComponent(key)}`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      isConnecting = false;
    }
  }

  async function handleQuickGuacamole(e: MouseEvent, server: ServerAttribute) {
    e.stopPropagation();
    isConnecting = true;
    connectingStatus = "Connecting to Guacamole…";
    errorMsg = "";
    try {
      const key = await connectToGuacamoleSession(server, (msg) => {
        connectingStatus = msg;
      });
      goto(`/remote?key=${encodeURIComponent(key)}`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      isConnecting = false;
    }
  }

  function handleEditSsh(e: MouseEvent, server: ServerAttribute) {
    e.stopPropagation();
    editModalServer = server;
    editModalOpen = true;
  }

  async function handleSaveServer(updated: ServerAttribute) {
    await updateServer(updated);
    servers = servers.map((s) => (s.id === updated.id ? updated : s));
    toast.success("Server updated.");
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
                bind:this={searchInputEl}
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
              <div class="card server-card" id="server-{server.id}">
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
                  <span class="os-badge">{server.os}</span>
                </div>

                <div class="card-meta">
                  <code class="user-ip">{server.ip}:{server.ssh_port}</code>
                  {#if server.description}
                    <span class="label-text">{server.description}</span>
                  {/if}
                </div>

                <div class="card-actions">
                  {#if server.os === "windows"}
                    <button
                      class="action-btn primary"
                      title="Quick connect via Guacamole using your Default Account"
                      onclick={(e) => handleQuickGuacamole(e, server)}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <rect x="2" y="3" width="20" height="14" rx="2" />
                        <line x1="8" y1="21" x2="16" y2="21" />
                        <line x1="12" y1="17" x2="12" y2="21" />
                      </svg>
                      Guacamole
                    </button>
                  {:else}
                    <button
                      class="action-btn primary"
                      title="Quick connect via SSH using your Default Account"
                      onclick={(e) => handleQuickSsh(e, server)}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
                      </svg>
                      SSH
                    </button>
                  {/if}
                  <button
                    class="action-btn primary"
                    title="Quick connect via SFTP using your Default Account"
                    onclick={(e) => handleQuickSftp(e, server)}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
                    </svg>
                    SFTP
                  </button>
                  <button
                    class="action-btn"
                    title="Edit SSH connection"
                    onclick={(e) => handleEditSsh(e, server)}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                      <path d="M18.5 2.5a2.12 2.12 0 0 1 3 3L12 15l-4 1 1-4z"/>
                    </svg>
                  </button>
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

  <EditServerModal
    isOpen={editModalOpen}
    server={editModalServer}
    onClose={() => (editModalOpen = false)}
    onSave={handleSaveServer}
  />
</main>

<style>
  /* ── Layout ──────────────────────────────────────────────── */
  .main-container {
    width: 100%;
    height: 100%;
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
    cursor: default;
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

  .os-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--sf-text-hint);
    background: var(--sf-bg-hover);
    border: 1px solid var(--sf-border);
    border-radius: var(--sf-radius-pill);
    padding: 2px 8px;
    flex-shrink: 0;
  }

  .card-actions {
    display: flex;
    gap: 6px;
    border-top: 1px solid rgba(26, 51, 82, 0.6);
    padding-top: 12px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    background: var(--sf-bg-hover);
    border: 1px solid var(--sf-border);
    border-radius: var(--sf-radius-sm);
    color: var(--sf-text-secondary);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    padding: 6px 8px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .action-btn:not(.primary) {
    flex: 0 0 auto;
    padding: 6px 9px;
  }

  .action-btn.primary {
    flex: 1;
  }

  .action-btn:hover {
    border-color: var(--sf-accent);
    color: var(--sf-accent);
    background: var(--sf-accent-dim);
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
