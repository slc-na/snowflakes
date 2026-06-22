<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open, save } from "@tauri-apps/plugin-dialog";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import type { SessionInfo } from "../../types/settings";

    let {
        sessionKey,
        sessionInfo,
        onClose,
    }: {
        sessionKey: string;
        sessionInfo: SessionInfo;
        onClose: () => void;
    } = $props();

    type SftpEntry = {
        name: string;
        path: string;
        is_dir: boolean;
        size: number;
        modified: number;
        permissions?: string;
    };

    type TreeNode = {
        name: string;
        path: string;
        isOpen: boolean;
        isLoaded: boolean;
        children: TreeNode[];
    };

    type TransferItem = {
        id: string;
        name: string;
        direction: "upload" | "download";
        progress: number;
        status: "running" | "done" | "error";
        error?: string;
    };
    let selectedFile = $state<SftpEntry | null>(null);
    let isLoading = $state(true);
    let loadError = $state("");
    let searchQuery = $state("");
    // ── State ──────────────────────────────────────────────────────────────────
    let currentPath = $state("/");
    let files = $state<SftpEntry[]>([]);
    let filteredFiles = $derived(
        searchQuery.trim()
            ? files.filter((f) =>
                  f.name.toLowerCase().includes(searchQuery.toLowerCase()),
              )
            : files,
    );

    let tree = $state<TreeNode[]>([
        {
            name: "/",
            path: "/",
            isOpen: false,
            isLoaded: false,
            children: [],
        },
    ]);

    let transfers = $state<TransferItem[]>([]);
    let unlisteners: UnlistenFn[] = [];

    let showNewFolderInput = $state(false);
    let newFolderName = $state("");
    let isCreatingFolder = $state(false);

    let isDragging = $state(false);
    let sortBy = $state<"name" | "size" | "modified">("name");
    let sortAsc = $state(true);

    let sortedFiles = $derived(
        [...filteredFiles].sort((a, b) => {
            // Folder selalu di atas
            if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
            let cmp = 0;
            if (sortBy === "name") cmp = a.name.localeCompare(b.name);
            else if (sortBy === "size") cmp = a.size - b.size;
            else if (sortBy === "modified") cmp = a.modified - b.modified;
            return sortAsc ? cmp : -cmp;
        }),
    );

    let breadcrumbs = $derived(
        currentPath === "/"
            ? ["/"]
            : ["", ...currentPath.split("/").filter(Boolean)],
    );

    onMount(async () => {
        await navigateTo("/");
        await loadTreeNode(tree[0]);
        tree[0].isOpen = true;
    });

    onDestroy(() => {
        unlisteners.forEach((fn) => fn());
        // Don't auto-disconnect here - this fires on every unmount, including
        // just navigating away to another tab. The SFTP session should stay
        // alive in the backend (like SSH sessions do) until the user explicitly
        // closes it via onClose, which already calls disconnectSftp itself.
    });

    async function navigateTo(path: string) {
        isLoading = true;
        loadError = "";
        selectedFile = null;
        currentPath = path;
        try {
            const result = await invoke<SftpEntry[]>("sftp_list_dir", {
                sessionKey,
                remotePath: path,
            });
            files = result;
        } catch (err) {
            loadError = String(err);
            files = [];
        } finally {
            isLoading = false;
        }
    }

    function navigateToBreadcrumb(index: number) {
        if (index === 0) {
            navigateTo("/");
            return;
        }
        const parts = currentPath.split("/").filter(Boolean);
        const newPath = "/" + parts.slice(0, index).join("/");
        navigateTo(newPath);
    }

    function handleFileDoubleClick(file: SftpEntry) {
        if (file.is_dir) {
            navigateTo(file.path);
            syncTreeToPath(file.path);
        }
    }

    async function loadTreeNode(node: TreeNode) {
        if (node.isLoaded) return;
        try {
            const result = await invoke<SftpEntry[]>("sftp_list_dir", {
                sessionKey,
                remotePath: node.path,
            });
            node.children = result
                .filter((e) => e.is_dir)
                .map((e) => ({
                    name: e.name,
                    path: e.path,
                    isOpen: false,
                    isLoaded: false,
                    children: [],
                }));
            node.isLoaded = true;
        } catch {
            node.isLoaded = true;
        }
        tree = [...tree]; // trigger reactivity
    }

    async function toggleTreeNode(node: TreeNode) {
        if (!node.isLoaded) await loadTreeNode(node);
        node.isOpen = !node.isOpen;
        tree = [...tree];
        if (node.isOpen) {
            navigateTo(node.path);
        }
    }

    function syncTreeToPath(_path: string) {
        // Best effort — expand tree to match current path
        // (untuk simplicity, tree sudah sync via navigateTo)
    }

    function isTreeNodeActive(node: TreeNode): boolean {
        return currentPath === node.path;
    }

    async function handleUpload() {
        try {
            const selected = await open({ multiple: true });
            if (!selected) return;
            const paths = Array.isArray(selected) ? selected : [selected];
            for (const localPath of paths) {
                const fileName = localPath.split(/[\\/]/).pop() ?? localPath;
                const remotePath = currentPath.endsWith("/")
                    ? `${currentPath}${fileName}`
                    : `${currentPath}/${fileName}`;
                await startUpload(localPath, remotePath, fileName);
            }
        } catch (err) {
            console.error("Upload failed:", err);
        }
    }

    async function startUpload(
        localPath: string,
        remotePath: string,
        fileName: string,
    ) {
        const id = crypto.randomUUID();
        transfers = [
            ...transfers,
            {
                id,
                name: fileName,
                direction: "upload",
                progress: 0,
                status: "running",
            },
        ];

        const unlisten = await listen<number>(
            `sftp-upload-progress-${sessionKey}`,
            (e) => {
                updateTransferProgress(id, e.payload);
            },
        );
        unlisteners.push(unlisten);

        const doneUnlisten = await listen<string>(
            `sftp-upload-done-${sessionKey}`,
            () => {
                completeTransfer(id);
                navigateTo(currentPath); // refresh listing
            },
        );
        unlisteners.push(doneUnlisten);

        try {
            await invoke("sftp_upload_file", {
                window: null,
                sessionKey,
                localPath,
                remotePath,
            });
        } catch (err) {
            failTransfer(id, String(err));
        }
    }

    async function handleDownload(file: SftpEntry) {
        if (file.is_dir) return;
        try {
            const localPath = await save({ defaultPath: file.name });
            if (!localPath) return;
            await startDownload(file.path, localPath, file.name);
        } catch (err) {
            console.error("Download failed:", err);
        }
    }

    async function startDownload(
        remotePath: string,
        localPath: string,
        fileName: string,
    ) {
        const id = crypto.randomUUID();
        transfers = [
            ...transfers,
            {
                id,
                name: fileName,
                direction: "download",
                progress: 0,
                status: "running",
            },
        ];

        const unlisten = await listen<number>(
            `sftp-download-progress-${sessionKey}`,
            (e) => {
                updateTransferProgress(id, e.payload);
            },
        );
        unlisteners.push(unlisten);

        const doneUnlisten = await listen<string>(
            `sftp-download-done-${sessionKey}`,
            () => {
                completeTransfer(id);
            },
        );
        unlisteners.push(doneUnlisten);

        try {
            await invoke("sftp_download_file", {
                window: null,
                sessionKey,
                remotePath,
                localPath,
            });
        } catch (err) {
            failTransfer(id, String(err));
        }
    }

    function updateTransferProgress(id: string, progress: number) {
        transfers = transfers.map((t) =>
            t.id === id ? { ...t, progress } : t,
        );
    }

    function completeTransfer(id: string) {
        transfers = transfers.map((t) =>
            t.id === id ? { ...t, progress: 100, status: "done" } : t,
        );
        setTimeout(() => {
            transfers = transfers.filter((t) => t.id !== id);
        }, 3000);
    }

    function failTransfer(id: string, error: string) {
        transfers = transfers.map((t) =>
            t.id === id ? { ...t, status: "error", error } : t,
        );
    }

    async function createFolder() {
        if (!newFolderName.trim()) return;
        isCreatingFolder = true;
        const path = currentPath.endsWith("/")
            ? `${currentPath}${newFolderName.trim()}`
            : `${currentPath}/${newFolderName.trim()}`;
        try {
            await invoke("sftp_mkdir", { sessionKey, remotePath: path });
            await navigateTo(currentPath);
            newFolderName = "";
            showNewFolderInput = false;
        } catch (err) {
            loadError = String(err);
        } finally {
            isCreatingFolder = false;
        }
    }

    // ── Drag & Drop upload ─────────────────────────────────────────────────────
    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        isDragging = true;
    }

    function handleDragLeave() {
        isDragging = false;
    }

    async function handleDrop(e: DragEvent) {
        e.preventDefault();
        isDragging = false;
        const items = e.dataTransfer?.files;
        if (!items) return;
        // Note: Tauri drag-drop gives us file paths via tauri-specific events.
        // Untuk web file object, kita bisa minta user gunakan dialog.
        // Di sini kita fallback ke dialog jika drop tidak support path.
        handleUpload();
    }

    // ── Sort ───────────────────────────────────────────────────────────────────
    function toggleSort(col: "name" | "size" | "modified") {
        if (sortBy === col) {
            sortAsc = !sortAsc;
        } else {
            sortBy = col;
            sortAsc = true;
        }
    }

    // ── Utilities ──────────────────────────────────────────────────────────────
    function formatSize(bytes: number): string {
        if (bytes === 0) return "--";
        const units = ["B", "KB", "MB", "GB", "TB"];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
    }

    function formatDate(ts: number): string {
        if (!ts) return "--";
        const d = new Date(ts * 1000);
        const now = new Date();
        const diffDays = Math.floor(
            (now.getTime() - d.getTime()) / (1000 * 60 * 60 * 24),
        );
        if (diffDays === 0)
            return `Today, ${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
        if (diffDays === 1)
            return `Yesterday, ${d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
        return d.toLocaleDateString([], {
            day: "numeric",
            month: "short",
            year: "numeric",
        });
    }

    function fileIconType(entry: SftpEntry): string {
        if (entry.is_dir) return "folder";
        const ext = entry.name.split(".").pop()?.toLowerCase() ?? "";
        if (["png", "jpg", "jpeg", "gif", "webp", "svg"].includes(ext))
            return "image";
        if (["tar", "gz", "zip", "bz2", "xz", "7z", "rar"].includes(ext))
            return "archive";
        if (
            [
                "yml",
                "yaml",
                "json",
                "ts",
                "js",
                "rs",
                "go",
                "py",
                "sh",
                "conf",
                "toml",
                "env",
            ].includes(ext)
        )
            return "code";
        return "file";
    }

    function handleNewFolderKey(e: KeyboardEvent) {
        if (e.key === "Enter") createFolder();
        if (e.key === "Escape") {
            showNewFolderInput = false;
            newFolderName = "";
        }
    }
</script>

<div class="fs-overlay">
    <div class="fs-layout">
        <header class="fs-header">
            <div class="header-left">
                <button
                    class="btn btn-ghost back-btn"
                    onclick={onClose}
                    title="Close SFTP"
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M19 12H5M5 12l7 7M5 12l7-7" />
                    </svg>
                </button>

                <div class="divider-v"></div>

                <!-- Connection badge -->
                <div class="conn-badge">
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <rect x="2" y="2" width="20" height="8" rx="2" /><rect
                            x="2"
                            y="14"
                            width="20"
                            height="8"
                            rx="2"
                        />
                        <line x1="6" y1="6" x2="6.01" y2="6" /><line
                            x1="6"
                            y1="18"
                            x2="6.01"
                            y2="18"
                        />
                    </svg>
                    <span class="conn-host">{sessionInfo.targetIp}</span>
                    <span class="conn-user">{sessionInfo.username}</span>
                    <span class="conn-dot"></span>
                    <span class="conn-label">SFTP</span>
                </div>

                <div class="divider-v"></div>

                <!-- Breadcrumbs -->
                <nav class="breadcrumbs">
                    {#each breadcrumbs as crumb, i}
                        {#if i > 0}
                            <svg
                                width="12"
                                height="12"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                class="crumb-sep"
                            >
                                <path d="M9 18l6-6-6-6" />
                            </svg>
                        {/if}
                        <button
                            class="crumb {i === breadcrumbs.length - 1
                                ? 'crumb-active'
                                : ''}"
                            onclick={() => navigateToBreadcrumb(i)}
                        >
                            {crumb === "" ? "/" : crumb}
                        </button>
                    {/each}
                </nav>
            </div>

            <div class="header-right">
                <!-- Search -->
                <div class="search-wrap">
                    <svg
                        width="13"
                        height="13"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        class="search-icon"
                    >
                        <circle cx="11" cy="11" r="8" /><path
                            d="M21 21l-4.35-4.35"
                        />
                    </svg>
                    <input
                        type="text"
                        placeholder="Filter files..."
                        bind:value={searchQuery}
                        class="search-input"
                    />
                    {#if searchQuery}
                        <button
                            class="search-clear"
                            onclick={() => (searchQuery = "")}>×</button
                        >
                    {/if}
                </div>

                <!-- New Folder -->
                <button
                    class="btn btn-ghost"
                    onclick={() => (showNewFolderInput = !showNewFolderInput)}
                    title="New Folder"
                >
                    <svg
                        width="15"
                        height="15"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path
                            d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                        />
                        <line x1="12" y1="11" x2="12" y2="17" /><line
                            x1="9"
                            y1="14"
                            x2="15"
                            y2="14"
                        />
                    </svg>
                </button>

                <!-- Upload -->
                <button class="btn btn-primary" onclick={handleUpload}>
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="17 8 12 3 7 8" /><line
                            x1="12"
                            y1="3"
                            x2="12"
                            y2="15"
                        />
                    </svg>
                    <span>Upload</span>
                </button>

                <!-- Refresh -->
                <button
                    class="btn btn-ghost"
                    onclick={() => navigateTo(currentPath)}
                    title="Refresh"
                >
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <polyline points="23 4 23 10 17 10" /><polyline
                            points="1 20 1 14 7 14"
                        />
                        <path
                            d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
                        />
                    </svg>
                </button>
            </div>
        </header>

        {#if showNewFolderInput}
            <div class="new-folder-bar">
                <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path
                        d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                    />
                </svg>
                <input
                    type="text"
                    placeholder="New folder name..."
                    bind:value={newFolderName}
                    onkeydown={handleNewFolderKey}
                    class="new-folder-input"
                    autofocus
                />
                <button
                    class="btn btn-primary btn-sm"
                    onclick={createFolder}
                    disabled={isCreatingFolder || !newFolderName.trim()}
                >
                    {isCreatingFolder ? "Creating..." : "Create"}
                </button>
                <button
                    class="btn btn-ghost btn-sm"
                    onclick={() => {
                        showNewFolderInput = false;
                        newFolderName = "";
                    }}
                >
                    Cancel
                </button>
            </div>
        {/if}

        <div class="fs-body">
            <!-- Tree Sidebar -->
            <aside class="fs-sidebar">
                <div class="sidebar-section">
                    <p class="section-label">REMOTE FILESYSTEM</p>
                    <div class="tree">
                        {#each tree as node}
                            {@render treeNodeItem(node, 0)}
                        {/each}
                    </div>
                </div>

                <!-- Transfer queue -->
                {#if transfers.length > 0}
                    <div class="sidebar-section transfers-section">
                        <p class="section-label">TRANSFERS</p>
                        <div class="transfer-list">
                            {#each transfers as t (t.id)}
                                <div class="transfer-item">
                                    <div class="transfer-header">
                                        <span class="transfer-icon">
                                            {#if t.direction === "upload"}
                                                <svg
                                                    width="11"
                                                    height="11"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2.5"
                                                >
                                                    <polyline
                                                        points="17 8 12 3 7 8"
                                                    /><line
                                                        x1="12"
                                                        y1="3"
                                                        x2="12"
                                                        y2="15"
                                                    />
                                                    <path
                                                        d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                                                    />
                                                </svg>
                                            {:else}
                                                <svg
                                                    width="11"
                                                    height="11"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2.5"
                                                >
                                                    <path
                                                        d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                                                    />
                                                    <polyline
                                                        points="7 10 12 15 17 10"
                                                    /><line
                                                        x1="12"
                                                        y1="15"
                                                        x2="12"
                                                        y2="3"
                                                    />
                                                </svg>
                                            {/if}
                                        </span>
                                        <span class="transfer-name"
                                            >{t.name}</span
                                        >
                                        <span
                                            class="transfer-status status-{t.status}"
                                        >
                                            {t.status === "running"
                                                ? `${t.progress}%`
                                                : t.status === "done"
                                                  ? "Done"
                                                  : "Error"}
                                        </span>
                                    </div>
                                    {#if t.status === "running"}
                                        <div class="transfer-bar">
                                            <div
                                                class="transfer-fill"
                                                style="width: {t.progress}%"
                                            ></div>
                                        </div>
                                    {:else if t.status === "error" && t.error}
                                        <p class="transfer-error">{t.error}</p>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/if}
            </aside>

            <!-- Main file list -->
            <main
                class="fs-main {isDragging ? 'drag-over' : ''}"
                ondragover={handleDragOver}
                ondragleave={handleDragLeave}
                ondrop={handleDrop}
            >
                {#if isDragging}
                    <div class="drop-overlay">
                        <svg
                            width="40"
                            height="40"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        >
                            <path
                                d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                            />
                            <polyline points="17 8 12 3 7 8" /><line
                                x1="12"
                                y1="3"
                                x2="12"
                                y2="15"
                            />
                        </svg>
                        <p>Drop files to upload to {currentPath}</p>
                    </div>
                {:else if isLoading}
                    <!-- Skeleton -->
                    <div class="list-header-row">
                        <div class="col-name">Name</div>
                        <div class="col-size">Size</div>
                        <div class="col-date">Modified</div>
                        <div class="col-perms">Permissions</div>
                        <div class="col-actions"></div>
                    </div>
                    {#each Array(8) as _}
                        <div class="file-row skeleton-row">
                            <div class="col-name">
                                <div class="skel-icon shimmer"></div>
                                <div
                                    class="skel-text shimmer"
                                    style="width: {50 + Math.random() * 120}px"
                                ></div>
                            </div>
                            <div class="col-size">
                                <div
                                    class="skel-text shimmer"
                                    style="width:48px"
                                ></div>
                            </div>
                            <div class="col-date">
                                <div
                                    class="skel-text shimmer"
                                    style="width:90px"
                                ></div>
                            </div>
                            <div class="col-perms">
                                <div
                                    class="skel-text shimmer"
                                    style="width:80px"
                                ></div>
                            </div>
                        </div>
                    {/each}
                {:else if loadError}
                    <div class="empty-state error-state">
                        <svg
                            width="36"
                            height="36"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        >
                            <circle cx="12" cy="12" r="10" /><line
                                x1="15"
                                y1="9"
                                x2="9"
                                y2="15"
                            /><line x1="9" y1="9" x2="15" y2="15" />
                        </svg>
                        <p>{loadError}</p>
                        <button
                            class="btn btn-outline"
                            onclick={() => navigateTo(currentPath)}
                            >Retry</button
                        >
                    </div>
                {:else if sortedFiles.length === 0}
                    <div class="empty-state">
                        <svg
                            width="36"
                            height="36"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        >
                            <path
                                d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                            />
                        </svg>
                        <p>
                            {searchQuery
                                ? "No files match your search"
                                : "This folder is empty"}
                        </p>
                    </div>
                {:else}
                    <!-- Column headers -->
                    <div class="list-header-row">
                        <button
                            class="col-name col-sort {sortBy === 'name'
                                ? 'sort-active'
                                : ''}"
                            onclick={() => toggleSort("name")}
                        >
                            Name {#if sortBy === "name"}<span class="sort-arrow"
                                    >{sortAsc ? "↑" : "↓"}</span
                                >{/if}
                        </button>
                        <button
                            class="col-size col-sort {sortBy === 'size'
                                ? 'sort-active'
                                : ''}"
                            onclick={() => toggleSort("size")}
                        >
                            Size {#if sortBy === "size"}<span class="sort-arrow"
                                    >{sortAsc ? "↑" : "↓"}</span
                                >{/if}
                        </button>
                        <button
                            class="col-date col-sort {sortBy === 'modified'
                                ? 'sort-active'
                                : ''}"
                            onclick={() => toggleSort("modified")}
                        >
                            Modified {#if sortBy === "modified"}<span
                                    class="sort-arrow"
                                    >{sortAsc ? "↑" : "↓"}</span
                                >{/if}
                        </button>
                        <div class="col-perms">Permissions</div>
                        <div class="col-actions"></div>
                    </div>

                    <div class="file-list">
                        {#each sortedFiles as file (file.path)}
                            {@const iconType = fileIconType(file)}
                            <div
                                class="file-row {selectedFile?.path ===
                                file.path
                                    ? 'selected'
                                    : ''}"
                                onclick={() => (selectedFile = file)}
                                ondblclick={() => handleFileDoubleClick(file)}
                                role="row"
                                tabindex="0"
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    handleFileDoubleClick(file)}
                            >
                                <div class="col-name">
                                    <span class="file-icon icon-{iconType}">
                                        {#if iconType === "folder"}
                                            <svg
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="currentColor"
                                                stroke="none"
                                            >
                                                <path
                                                    d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                                                    opacity="0.3"
                                                />
                                                <path
                                                    d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="1.5"
                                                />
                                            </svg>
                                        {:else if iconType === "image"}
                                            <svg
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                            >
                                                <rect
                                                    x="3"
                                                    y="3"
                                                    width="18"
                                                    height="18"
                                                    rx="2"
                                                /><circle
                                                    cx="8.5"
                                                    cy="8.5"
                                                    r="1.5"
                                                />
                                                <polyline
                                                    points="21 15 16 10 5 21"
                                                />
                                            </svg>
                                        {:else if iconType === "archive"}
                                            <svg
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                            >
                                                <polyline
                                                    points="21 8 21 21 3 21 3 8"
                                                /><rect
                                                    x="1"
                                                    y="3"
                                                    width="22"
                                                    height="5"
                                                />
                                                <line
                                                    x1="10"
                                                    y1="12"
                                                    x2="14"
                                                    y2="12"
                                                />
                                            </svg>
                                        {:else if iconType === "code"}
                                            <svg
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                            >
                                                <polyline
                                                    points="16 18 22 12 16 6"
                                                /><polyline
                                                    points="8 6 2 12 8 18"
                                                />
                                            </svg>
                                        {:else}
                                            <svg
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="1.5"
                                            >
                                                <path
                                                    d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                                />
                                                <polyline
                                                    points="14 2 14 8 20 8"
                                                />
                                            </svg>
                                        {/if}
                                    </span>
                                    <span class="file-name">{file.name}</span>
                                </div>
                                <div class="col-size">
                                    {file.is_dir ? "--" : formatSize(file.size)}
                                </div>
                                <div class="col-date">
                                    {formatDate(file.modified)}
                                </div>
                                <div class="col-perms">
                                    {#if file.permissions}
                                        <span class="perms-badge"
                                            >{file.permissions}</span
                                        >
                                    {/if}
                                </div>
                                <div class="col-actions">
                                    {#if !file.is_dir}
                                        <button
                                            class="action-btn"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                handleDownload(file);
                                            }}
                                            title="Download"
                                        >
                                            <svg
                                                width="13"
                                                height="13"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                            >
                                                <path
                                                    d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                                                />
                                                <polyline
                                                    points="7 10 12 15 17 10"
                                                /><line
                                                    x1="12"
                                                    y1="15"
                                                    x2="12"
                                                    y2="3"
                                                />
                                            </svg>
                                        </button>
                                    {:else}
                                        <button
                                            class="action-btn"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                navigateTo(file.path);
                                            }}
                                            title="Open folder"
                                        >
                                            <svg
                                                width="13"
                                                height="13"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                            >
                                                <path
                                                    d="M5 12h14M12 5l7 7-7 7"
                                                />
                                            </svg>
                                        </button>
                                    {/if}
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}
            </main>

            <!-- Details Pane -->
            <aside class="fs-details">
                {#if selectedFile}
                    {@const iconType = fileIconType(selectedFile)}
                    <div class="details-header">
                        <div class="preview-box icon-{iconType}-lg">
                            {#if iconType === "folder"}
                                <svg
                                    width="44"
                                    height="44"
                                    viewBox="0 0 24 24"
                                    fill="currentColor"
                                    stroke="none"
                                >
                                    <path
                                        d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                                        opacity="0.35"
                                    />
                                    <path
                                        d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="1.2"
                                    />
                                </svg>
                            {:else if iconType === "image"}
                                <svg
                                    width="44"
                                    height="44"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.2"
                                >
                                    <rect
                                        x="3"
                                        y="3"
                                        width="18"
                                        height="18"
                                        rx="2"
                                    /><circle cx="8.5" cy="8.5" r="1.5" />
                                    <polyline points="21 15 16 10 5 21" />
                                </svg>
                            {:else if iconType === "archive"}
                                <svg
                                    width="44"
                                    height="44"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.2"
                                >
                                    <polyline
                                        points="21 8 21 21 3 21 3 8"
                                    /><rect x="1" y="3" width="22" height="5" />
                                    <line x1="10" y1="12" x2="14" y2="12" />
                                </svg>
                            {:else if iconType === "code"}
                                <svg
                                    width="44"
                                    height="44"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.2"
                                >
                                    <polyline
                                        points="16 18 22 12 16 6"
                                    /><polyline points="8 6 2 12 8 18" />
                                </svg>
                            {:else}
                                <svg
                                    width="44"
                                    height="44"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.2"
                                >
                                    <path
                                        d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                                    />
                                    <polyline points="14 2 14 8 20 8" />
                                </svg>
                            {/if}
                        </div>
                        <h2 class="detail-name">{selectedFile.name}</h2>
                        <span class="detail-type"
                            >{selectedFile.is_dir
                                ? "FOLDER"
                                : (selectedFile.name
                                      .split(".")
                                      .pop()
                                      ?.toUpperCase() ?? "FILE")}</span
                        >
                    </div>

                    <div class="details-info">
                        <div class="info-row">
                            <span class="info-label">Path</span>
                            <span class="info-value mono path-value"
                                >{selectedFile.path}</span
                            >
                        </div>
                        <div class="info-row">
                            <span class="info-label">Size</span>
                            <span class="info-value"
                                >{selectedFile.is_dir
                                    ? "--"
                                    : formatSize(selectedFile.size)}</span
                            >
                        </div>
                        <div class="info-row">
                            <span class="info-label">Modified</span>
                            <span class="info-value"
                                >{formatDate(selectedFile.modified)}</span
                            >
                        </div>
                        {#if selectedFile.permissions}
                            <div class="info-row">
                                <span class="info-label">Permissions</span>
                                <span class="info-value mono accent"
                                    >{selectedFile.permissions}</span
                                >
                            </div>
                        {/if}
                    </div>

                    <div class="details-actions">
                        {#if !selectedFile.is_dir}
                            <button
                                class="btn btn-outline full-w"
                                onclick={() => handleDownload(selectedFile!)}
                            >
                                <svg
                                    width="13"
                                    height="13"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                >
                                    <path
                                        d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                                    />
                                    <polyline points="7 10 12 15 17 10" /><line
                                        x1="12"
                                        y1="15"
                                        x2="12"
                                        y2="3"
                                    />
                                </svg>
                                Download
                            </button>
                        {:else}
                            <button
                                class="btn btn-outline full-w"
                                onclick={() => navigateTo(selectedFile!.path)}
                            >
                                <svg
                                    width="13"
                                    height="13"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                >
                                    <path d="M5 12h14M12 5l7 7-7 7" />
                                </svg>
                                Open folder
                            </button>
                        {/if}
                    </div>
                {:else}
                    <div class="details-empty">
                        <svg
                            width="28"
                            height="28"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.2"
                        >
                            <path
                                d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                            />
                            <polyline points="14 2 14 8 20 8" />
                        </svg>
                        <p>Select a file to see details</p>
                    </div>
                {/if}
            </aside>
        </div>
    </div>
</div>

<!-- ── Tree node snippet ────────────────────────────────────────────────────── -->
{#snippet treeNodeItem(node: TreeNode, depth: number)}
    <div class="tree-node" style="padding-left: {depth * 14}px">
        <button
            class="tree-item {isTreeNodeActive(node) ? 'tree-active' : ''}"
            onclick={() => {
                toggleTreeNode(node);
                navigateTo(node.path);
            }}
        >
            <span class="tree-chevron">
                {#if node.children.length > 0 || !node.isLoaded}
                    {#if node.isOpen}
                        <svg
                            width="12"
                            height="12"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"><path d="M6 9l6 6 6-6" /></svg
                        >
                    {:else}
                        <svg
                            width="12"
                            height="12"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"><path d="M9 6l6 6-6 6" /></svg
                        >
                    {/if}
                {:else}
                    <span style="width:12px;display:inline-block"></span>
                {/if}
            </span>
            <span class="tree-folder-icon">
                {#if node.isOpen}
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                        stroke="none"
                    >
                        <path
                            d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                            opacity="0.3"
                        />
                        <path
                            d="M20 6h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2z"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                {:else}
                    <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                    >
                        <path
                            d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
                        />
                    </svg>
                {/if}
            </span>
            <span class="tree-label">{node.name}</span>
        </button>
        {#if node.isOpen}
            {#each node.children as child}
                {@render treeNodeItem(child, depth + 1)}
            {/each}
        {/if}
    </div>
{/snippet}

<style>
    /* ── Overlay / Layout ────────────────────────────── */
    /* Positioned absolute (not fixed) so it's scoped to the page's content
       area (its nearest positioned ancestor) instead of covering the whole
       viewport - keeps the sidebar and session tab bar visible above it. */
    .fs-overlay {
        position: absolute;
        inset: 0;
        z-index: 50;
        background: var(--sf-bg-app);
        display: flex;
        flex-direction: column;
        animation: fade-in 0.2s ease;
    }

    .fs-layout {
        display: flex;
        flex-direction: column;
        height: 100%;
        overflow: hidden;
    }

    @keyframes fade-in {
        from {
            opacity: 0;
            transform: translateY(6px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    /* ── Header ──────────────────────────────────────── */
    .fs-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        height: 52px;
        padding: 0 16px;
        background: var(--sf-bg-surface);
        border-bottom: 1px solid var(--sf-border);
        flex-shrink: 0;
        gap: 12px;
    }

    .header-left,
    .header-right {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .header-left {
        flex: 1;
        min-width: 0;
        overflow: hidden;
    }

    .divider-v {
        width: 1px;
        height: 20px;
        background: var(--sf-border);
        flex-shrink: 0;
    }

    /* Connection badge */
    .conn-badge {
        display: flex;
        align-items: center;
        gap: 7px;
        padding: 5px 10px;
        background: var(--sf-bg-app);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-md);
        flex-shrink: 0;
    }

    .conn-host {
        font-family: var(--sf-font-mono);
        font-size: 11px;
        color: var(--sf-text-primary);
        font-weight: 600;
    }

    .conn-user {
        font-size: 11px;
        color: var(--sf-text-secondary);
    }

    .conn-dot {
        width: 5px;
        height: 5px;
        background: var(--sf-online);
        border-radius: 50%;
        flex-shrink: 0;
    }

    .conn-label {
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.08em;
        color: #9ccfd8;
        text-transform: uppercase;
    }

    /* Breadcrumbs */
    .breadcrumbs {
        display: flex;
        align-items: center;
        gap: 2px;
        overflow: hidden;
    }

    .crumb {
        background: none;
        border: none;
        font-size: 12px;
        color: var(--sf-text-secondary);
        cursor: pointer;
        padding: 3px 6px;
        border-radius: 4px;
        white-space: nowrap;
        transition: all 0.15s;
    }

    .crumb:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-text-primary);
    }
    .crumb-active {
        color: var(--sf-text-primary) !important;
        font-weight: 500;
    }
    .crumb-sep {
        color: var(--sf-text-muted, #4a6a8a);
        flex-shrink: 0;
    }

    /* Search */
    .search-wrap {
        position: relative;
        display: flex;
        align-items: center;
    }

    .search-icon {
        position: absolute;
        left: 9px;
        color: var(--sf-text-muted, #4a6a8a);
        pointer-events: none;
    }

    .search-input {
        width: 180px;
        height: 30px;
        background: var(--sf-bg-input, var(--sf-bg-app));
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-md);
        padding: 0 28px 0 30px;
        color: var(--sf-text-primary);
        font-size: 12px;
        outline: none;
        transition: all 0.2s;
        font-family: var(--sf-font-ui);
    }

    .search-input:focus {
        border-color: var(--sf-border-focus);
        width: 220px;
    }

    .search-clear {
        position: absolute;
        right: 8px;
        background: none;
        border: none;
        color: var(--sf-text-secondary);
        cursor: pointer;
        font-size: 14px;
        line-height: 1;
    }

    /* Buttons */
    .btn {
        display: flex;
        align-items: center;
        gap: 6px;
        height: 30px;
        padding: 0 11px;
        font-size: 12px;
        font-weight: 500;
        border-radius: var(--sf-radius-md);
        cursor: pointer;
        transition: all 0.15s;
        border: none;
        outline: none;
        font-family: var(--sf-font-ui);
    }

    .btn-ghost {
        background: transparent;
        color: var(--sf-text-secondary);
        border: 1px solid var(--sf-border);
    }
    .btn-ghost:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-text-primary);
    }

    .btn-primary {
        background: var(--sf-accent);
        color: var(--sf-text-on-accent, #042c53);
    }
    .btn-primary:hover {
        filter: brightness(1.1);
    }
    .btn-primary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .btn-outline {
        background: transparent;
        color: var(--sf-text-primary);
        border: 1px solid var(--sf-border);
    }
    .btn-outline:hover {
        background: var(--sf-bg-hover);
    }

    .btn-sm {
        height: 26px;
        padding: 0 10px;
        font-size: 11px;
    }
    .full-w {
        width: 100%;
        justify-content: center;
    }

    .back-btn {
        width: 30px;
        padding: 0;
        justify-content: center;
        flex-shrink: 0;
    }

    /* New folder bar */
    .new-folder-bar {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 16px;
        background: var(--sf-bg-surface);
        border-bottom: 1px solid var(--sf-border);
        color: var(--sf-text-secondary);
        animation: slide-down 0.15s ease;
    }

    @keyframes slide-down {
        from {
            opacity: 0;
            transform: translateY(-6px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    .new-folder-input {
        flex: 1;
        height: 28px;
        background: var(--sf-bg-input, var(--sf-bg-app));
        border: 1px solid var(--sf-border-focus);
        border-radius: var(--sf-radius-md);
        padding: 0 10px;
        color: var(--sf-text-primary);
        font-size: 12px;
        outline: none;
        font-family: var(--sf-font-mono);
    }

    /* ── Body ────────────────────────────────────────── */
    .fs-body {
        display: flex;
        flex: 1;
        overflow: hidden;
    }

    /* ── Sidebar ─────────────────────────────────────── */
    .fs-sidebar {
        width: 240px;
        background: var(--sf-bg-app);
        border-right: 1px solid var(--sf-border);
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
        overflow-y: auto;
    }

    .sidebar-section {
        padding: 16px 12px;
    }
    .section-label {
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.08em;
        color: var(--sf-text-muted, #4a6a8a);
        margin: 0 0 10px 4px;
    }

    /* Tree */
    .tree {
        font-family: var(--sf-font-ui);
    }

    .tree-node {
        display: flex;
        flex-direction: column;
    }

    .tree-item {
        display: flex;
        align-items: center;
        gap: 5px;
        padding: 5px 8px;
        border-radius: var(--sf-radius-sm);
        cursor: pointer;
        transition: all 0.12s;
        background: none;
        border: none;
        color: var(--sf-text-secondary);
        width: 100%;
        text-align: left;
        font-size: 12px;
    }

    .tree-item:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-text-primary);
    }

    .tree-active {
        background: rgba(79, 195, 247, 0.1) !important;
        color: var(--sf-text-primary) !important;
    }

    .tree-chevron {
        color: var(--sf-text-muted, #4a6a8a);
        flex-shrink: 0;
        display: flex;
        align-items: center;
    }
    .tree-folder-icon {
        color: #f6c177;
        flex-shrink: 0;
        display: flex;
    }
    .tree-label {
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    /* Transfers */
    .transfers-section {
        border-top: 1px solid var(--sf-border);
        padding-top: 16px;
    }
    .transfer-list {
        display: flex;
        flex-direction: column;
        gap: 10px;
    }

    .transfer-item {
        background: var(--sf-bg-surface);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-sm);
        padding: 10px;
        display: flex;
        flex-direction: column;
        gap: 7px;
    }

    .transfer-header {
        display: flex;
        align-items: center;
        gap: 7px;
    }
    .transfer-icon {
        color: var(--sf-text-secondary);
        display: flex;
    }
    .transfer-name {
        font-size: 11px;
        color: var(--sf-text-primary);
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .transfer-status {
        font-size: 10px;
        font-weight: 600;
        flex-shrink: 0;
    }
    .status-running {
        color: var(--sf-accent);
    }
    .status-done {
        color: var(--sf-online);
    }
    .status-error {
        color: var(--sf-status-error);
    }

    .transfer-bar {
        height: 3px;
        background: var(--sf-bg-app);
        border-radius: 99px;
        overflow: hidden;
    }
    .transfer-fill {
        height: 100%;
        background: var(--sf-accent);
        border-radius: 99px;
        transition: width 0.3s ease;
    }

    .transfer-error {
        font-size: 10px;
        color: var(--sf-status-error);
        margin: 0;
    }

    /* ── Main file list ──────────────────────────────── */
    .fs-main {
        flex: 1;
        background: var(--sf-bg-app);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        position: relative;
        transition: background 0.15s;
    }

    .fs-main.drag-over {
        background: rgba(79, 195, 247, 0.05);
    }

    .drop-overlay {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 14px;
        background: rgba(79, 195, 247, 0.07);
        border: 2px dashed rgba(79, 195, 247, 0.4);
        z-index: 10;
        color: var(--sf-accent);
        pointer-events: none;
    }

    .drop-overlay p {
        font-size: 13px;
        font-weight: 500;
        margin: 0;
    }

    /* Column header */
    .list-header-row {
        display: grid;
        grid-template-columns: 3fr 1fr 2fr 1.2fr 60px;
        padding: 8px 20px;
        border-bottom: 1px solid var(--sf-border);
        flex-shrink: 0;
    }

    .col-sort {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: var(--sf-text-muted, #4a6a8a);
        padding: 0;
        text-align: left;
        display: flex;
        align-items: center;
        gap: 4px;
        transition: color 0.12s;
    }

    .col-sort:hover {
        color: var(--sf-text-secondary);
    }
    .sort-active {
        color: var(--sf-text-primary) !important;
    }
    .sort-arrow {
        font-size: 11px;
    }

    .col-size,
    .col-date,
    .col-perms,
    .col-actions {
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: var(--sf-text-muted, #4a6a8a);
        display: flex;
        align-items: center;
    }

    /* File list */
    .file-list {
        flex: 1;
        overflow-y: auto;
        padding: 6px;
    }

    .file-list::-webkit-scrollbar {
        width: 5px;
    }
    .file-list::-webkit-scrollbar-thumb {
        background: var(--sf-border);
        border-radius: 4px;
    }

    .file-row {
        display: grid;
        grid-template-columns: 3fr 1fr 2fr 1.2fr 60px;
        align-items: center;
        padding: 8px 14px;
        border-radius: var(--sf-radius-sm);
        cursor: pointer;
        transition: background 0.12s;
        border: 1px solid transparent;
    }

    .file-row:hover {
        background: var(--sf-bg-surface);
    }
    .file-row.selected {
        background: rgba(79, 195, 247, 0.07);
        border-color: var(--sf-border-focus, rgba(79, 195, 247, 0.3));
    }

    /* Column cells */
    .col-name {
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 13px;
        font-weight: 500;
        color: var(--sf-text-primary);
        overflow: hidden;
    }

    .file-name {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .col-size,
    .col-date {
        font-size: 11px;
        color: var(--sf-text-secondary);
        font-family: var(--sf-font-ui);
    }

    .perms-badge {
        font-family: var(--sf-font-mono);
        font-size: 10px;
        color: var(--sf-text-muted, #4a6a8a);
        background: var(--sf-bg-surface);
        padding: 3px 7px;
        border-radius: 4px;
        border: 1px solid var(--sf-border);
    }

    .col-actions {
        display: flex;
        justify-content: flex-end;
        opacity: 0;
        transition: opacity 0.12s;
    }
    .file-row:hover .col-actions {
        opacity: 1;
    }

    .action-btn {
        background: none;
        border: none;
        color: var(--sf-text-secondary);
        cursor: pointer;
        padding: 5px;
        border-radius: 4px;
        display: flex;
        transition: all 0.12s;
    }
    .action-btn:hover {
        background: var(--sf-bg-hover);
        color: var(--sf-accent);
    }

    /* File icons */
    .file-icon {
        display: flex;
        align-items: center;
        flex-shrink: 0;
    }
    .icon-folder {
        color: #f6c177;
    }
    .icon-image {
        color: #9ccfd8;
    }
    .icon-archive {
        color: #eb6f92;
    }
    .icon-code {
        color: #c4a7e7;
    }
    .icon-file {
        color: var(--sf-text-secondary);
    }

    /* Skeleton */
    .skeleton-row {
        pointer-events: none;
    }
    .skel-icon {
        width: 16px;
        height: 16px;
        border-radius: 3px;
        background: var(--sf-bg-surface);
        flex-shrink: 0;
    }
    .skel-text {
        height: 12px;
        border-radius: 3px;
        background: var(--sf-bg-surface);
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
            rgba(255, 255, 255, 0.04),
            transparent
        );
        animation: shimmer 1.5s infinite;
    }

    @keyframes shimmer {
        100% {
            transform: translateX(100%);
        }
    }

    /* Empty / error states */
    .empty-state {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 12px;
        color: var(--sf-text-muted, #4a6a8a);
        padding: 60px 20px;
    }
    .empty-state p {
        font-size: 13px;
        margin: 0;
        text-align: center;
    }
    .error-state {
        color: var(--sf-status-error);
    }

    /* ── Details Pane ────────────────────────────────── */
    .fs-details {
        width: 260px;
        background: var(--sf-bg-surface);
        border-left: 1px solid var(--sf-border);
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
        overflow-y: auto;
    }

    .details-header {
        padding: 28px 20px;
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        border-bottom: 1px solid var(--sf-border);
    }

    .preview-box {
        width: 80px;
        height: 80px;
        background: var(--sf-bg-app);
        border: 1px solid var(--sf-border);
        border-radius: var(--sf-radius-lg);
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 14px;
    }

    .icon-folder-lg {
        color: #f6c177;
    }
    .icon-image-lg {
        color: #9ccfd8;
    }
    .icon-archive-lg {
        color: #eb6f92;
    }
    .icon-code-lg {
        color: #c4a7e7;
    }
    .icon-file-lg {
        color: var(--sf-text-secondary);
    }

    .detail-name {
        font-size: 14px;
        font-weight: 600;
        color: var(--sf-text-primary);
        margin: 0 0 5px;
        word-break: break-all;
    }

    .detail-type {
        font-size: 10px;
        font-weight: 700;
        letter-spacing: 0.1em;
        color: var(--sf-text-secondary);
    }

    .details-info {
        padding: 20px;
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .info-row {
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .info-label {
        font-size: 10px;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: var(--sf-text-muted, #4a6a8a);
    }

    .info-value {
        font-size: 12px;
        color: var(--sf-text-primary);
        font-family: var(--sf-font-ui);
    }

    .mono {
        font-family: var(--sf-font-mono) !important;
    }
    .accent {
        color: var(--sf-accent) !important;
    }
    .path-value {
        word-break: break-all;
        font-size: 11px;
        color: var(--sf-text-secondary);
    }

    .details-actions {
        padding: 0 20px 20px;
        margin-top: auto;
    }

    .details-empty {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 10px;
        color: var(--sf-text-muted, #4a6a8a);
        padding: 40px 20px;
        text-align: center;
    }

    .details-empty p {
        font-size: 12px;
        margin: 0;
    }
</style>
