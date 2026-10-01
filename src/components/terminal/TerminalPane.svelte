<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { Terminal } from "@xterm/xterm";
    import { FitAddon } from "@xterm/addon-fit";
    import { SerializeAddon } from "@xterm/addon-serialize";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { invoke } from "@tauri-apps/api/core";
    import "@xterm/xterm/css/xterm.css";
    import ErrorModal from "../modal/ErrorModal.svelte";
    import type { SessionInfo } from "../../types/settings";
    import { loadSessionInfo } from "../../controller/local";
    import { deleteSession, reconnectToSession } from "../../controller/ssh";
    import {
        saveTerminalState,
        loadTerminalState,
    } from "../../controller/session";
    import { incrementTerminalFont, setTerminalFont } from "$lib/resizeTerminal";
    import { loadSettings } from "../../controller/local";

    let { targetKey, onClose } = $props<{
        targetKey: string;
        onClose: () => void;
    }>();
    let connectingStatus = $state("");
    let isLoading = $state(false);
    let session = $state<SessionInfo | null>(null);
    let terminalElement: HTMLElement;

    let unlisten: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let isOpenerrorMessage = $state(false);
    let errorMessage = $state("");

    let currentSshKey = $state<string | null>(null);
    let serializeAddon = $state<SerializeAddon | null>(null);
    let term: Terminal | null = null;
    let fitAddon: FitAddon | null = null;

    // Monotonic token identifying the "current" session in this pane. Any async
    // work started for an older token is stale and must not touch the terminal.
    let setupGeneration = 0;
    let cleanupPaneSession: (() => void) | null = null;

    // Dispose listeners/callbacks belonging to the currently attached session.
    // Called before (re)assigning a session and when the pane unmounts.
    function disposePaneSession(): void {
        if (unlisten) {
            unlisten();
            unlisten = undefined;
        }
        if (unlistenError) {
            unlistenError();
            unlistenError = undefined;
        }
        if (cleanupPaneSession) {
            cleanupPaneSession();
            cleanupPaneSession = null;
        }
    }

    onMount(() => {
        term = new Terminal({
            theme: {
                background: "#1a1b26",
                foreground: "#a9b1d6",
                cursor: "#f7768e",
            },
            cursorBlink: true,
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 13,
        });

        const now = new Date().toLocaleString();

        fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        serializeAddon = new SerializeAddon();
        term.loadAddon(serializeAddon);

        term.open(terminalElement);
        fitAddon.fit();
        term.focus();

        loadSettings().then((setting) => {
            if (term && fitAddon) setTerminalFont(term, fitAddon, setting.fontSize);
        });

        const setupSsh = async (key: string, generation: number) => {
            const activeTerm = term;
            const activeFitAddon = fitAddon;
            if (!activeTerm || !activeFitAddon) return;

            const isCurrent = () =>
                generation === setupGeneration && key === currentSshKey;

            isLoading = true;
            connectingStatus = "Initializing connection...";
            const currentSession = await loadSessionInfo(key);
            if (!isCurrent()) return;
            if (!currentSession) {
                activeTerm.writeln(
                    `\r\n\x1b[31mError: Session info not found for key: ${key}\x1b[0m`,
                );
                isLoading = false;
                return;
            }
            session = currentSession;

            const eventName = `ssh-output-${key}`;

            await reconnectToSession(session, (msg) => {
                if (!isCurrent()) return;
                connectingStatus = msg;
            });
            if (!isCurrent()) return;

            isLoading = false;

            const previousState = loadTerminalState(key);
            if (previousState) {
                activeTerm.write(previousState);
            } else {
                activeTerm.writeln("");
                activeTerm.writeln("========================================");
                activeTerm.writeln("          SSH CONNECTION INFO          ");
                activeTerm.writeln("----------------------------------------");
                activeTerm.writeln(` Session Key  : ${key}`);
                activeTerm.writeln(` Bastion Host : ${currentSession.bastionIp}`);
                activeTerm.writeln(` Username     : ${currentSession.username}`);
                activeTerm.writeln(` Target Host  : ${currentSession.targetIp}`);
                activeTerm.writeln(` Time         : ${now}`);
                activeTerm.writeln("----------------------------------------");
                activeTerm.writeln(` Press Enter to continue`);
                activeTerm.writeln("========================================");
                activeTerm.writeln("");
            }

            const localUnlisten = await listen(eventName, (event) => {
                if (!isCurrent()) return;
                activeTerm.write(event.payload as string);
            });
            if (!isCurrent()) {
                localUnlisten();
                return;
            }
            unlisten = localUnlisten;

            const errorEventName = `ssh-error-output-${key}`;

            const localUnlistenError = await listen(errorEventName, (event) => {
                if (!isCurrent()) return;
                isOpenerrorMessage = true;
                errorMessage = event.payload as string;
            });
            if (!isCurrent()) {
                localUnlisten();
                localUnlistenError();
                unlisten = undefined;
                return;
            }
            unlistenError = localUnlistenError;

            // forward xterm size changes to the remote pty so curses apps like
            // nano fill the split pane instead of the size the pty was born with
            const detachTermOnResize = activeTerm.onResize(({ cols, rows }) => {
                if (!isCurrent()) return;
                invoke("resize_ssh_pty", { cols, rows, ip: key }).catch((e) =>
                    console.debug("resize_ssh_pty failed", e),
                );
            });

            // the pty was created with the host/fallback size before this pane
            // existed, so push the real xterm size once here
            activeFitAddon.fit();
            if (activeTerm.cols > 0 && activeTerm.rows > 0) {
                invoke("resize_ssh_pty", {
                    cols: activeTerm.cols,
                    rows: activeTerm.rows,
                    ip: key,
                }).catch((e) => console.debug("resize_ssh_pty failed", e));
            }

            let inputBuffer = "";
            let debounceTimer: ReturnType<typeof setTimeout> | null = null;

            const detachTermOnData = activeTerm.onData((data: string) => {
                if (!isCurrent()) return;

                inputBuffer += data;

                if (debounceTimer) return;

                debounceTimer = setTimeout(() => {
                    const payload = inputBuffer;
                    inputBuffer = "";
                    debounceTimer = null;

                    if (!isCurrent()) return;

                    invoke("send_ssh_input", {
                        input: payload,
                        ip: key,
                    });
                }, 70);
            });

            cleanupPaneSession = () => {
                detachTermOnData.dispose();
                detachTermOnResize.dispose();
                if (debounceTimer) clearTimeout(debounceTimer);
            };
        };

        const handleResize = () => {
            // rAF lets the CSS grid/layout settle before FitAddon measures, and
            // avoids "ResizeObserver loop limit exceeded" while the pane reflows
            requestAnimationFrame(() => fitAddon?.fit());
        };

        const customKeyHandler = (e: KeyboardEvent) => {

            if (e.key === "Escape" && term) {
                e.preventDefault();
                term.blur();
                return false;
            }

            // Only the font-zoom shortcuts are consumed here. Every other key -
            // including Ctrl+C, Ctrl+D, Ctrl+L, ... - must fall through to xterm:
            // returning false makes xterm ignore the event, so the shell never sees it.
            if (e.type === 'keydown' && e.ctrlKey && term && fitAddon) {
                const increment =
                    e.key === "=" || e.key === "+" ? 1
                    : e.key === "-" ? -1
                    : 0;

                if (increment !== 0) {
                    e.preventDefault();
                    incrementTerminalFont(term, fitAddon, increment);
                    return false;
                }
            }
            return true;
        };
        term.attachCustomKeyEventHandler(customKeyHandler);

        // TODO : tambahin control + scroll

        $effect(() => {
            if (targetKey !== currentSshKey) {
                // invalidate any in-flight setup for the previous session first
                setupGeneration += 1;
                const generation = setupGeneration;
                disposePaneSession();

                if (targetKey && targetKey !== "unknown") {
                    // queued full reset (RIS): runs after the old session's pending
                    // output, unlike reset() which would run too early
                    term?.write("\x1bc");
                    currentSshKey = targetKey;
                    setupSsh(targetKey, generation);
                } else {
                    currentSshKey = "unknown";
                }
            }
        });

        // Window resize
        window.addEventListener("resize", handleResize);

        // Setup ResizeObserver for the terminal wrapper to trigger refit when layout changes
        const resizeObserver = new ResizeObserver(() => {
            handleResize();
        });
        resizeObserver.observe(terminalElement);
        if (terminalElement.parentElement) {
            resizeObserver.observe(terminalElement.parentElement);
        }

        // No global keydown so we don't zoom all terminals at once

        return () => {
            if (
                currentSshKey &&
                currentSshKey !== "unknown" &&
                serializeAddon
            ) {
                saveTerminalState(currentSshKey, serializeAddon.serialize());
            }
            setupGeneration += 1;
            disposePaneSession();
            window.removeEventListener("resize", handleResize);
            resizeObserver.disconnect();
            if (term) term.dispose();
        };
    });

    onDestroy(() => {
        if (currentSshKey && currentSshKey !== "unknown" && serializeAddon) {
            saveTerminalState(currentSshKey, serializeAddon.serialize());
        }
    });

    async function handleDisconnect() {
        if (targetKey && targetKey !== "unknown") {
            await deleteSession(targetKey);
        }
        onClose();
    }
</script>

<div class="flex flex-col w-full h-full bg-[#1a1b26] overflow-hidden">
    <div
        class="px-3 py-1 bg-[#16161e] border-b border-[#24283b] flex justify-between items-center shrink-0"
    >
        <div class="text-[10px] text-[#565f89] font-mono truncate">
            <span class="text-[#7aa2f7]">{session?.label || targetKey}</span>
            {#if session}
                <span class="mx-1 opacity-30">-</span>
                <span class="opacity-70"
                    >{session.username}@{session.targetIp}</span
                >
            {/if}
        </div>
        <button
            class="text-[10px] text-[#f7768e] hover:underline shrink-0 ml-2"
            onclick={handleDisconnect}
        >
            [x]
        </button>
    </div>

    <div class="flex-1 p-1 min-h-0 min-w-0 relative">
        <div bind:this={terminalElement} class="absolute inset-0 p-1"></div>
        
        {#if isLoading}
            <div class="pane-loading">
                <div class="spinner"></div>
                <span class="loading-msg">{connectingStatus}</span>
            </div>
        {/if}
    </div>
    <ErrorModal
        isOpen={isOpenerrorMessage}
        {errorMessage}
        sessionKey={currentSshKey}
    />
</div>

<style>
    .pane-loading {
        position: absolute;
        inset: 0;
        background: rgba(5, 15, 28, 0.85);
        backdrop-filter: blur(4px);
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        z-index: 10;
        gap: 12px;
    }

    .spinner {
        width: 32px;
        height: 32px;
        border: 2.5px solid var(--sf-border, #1a3352);
        border-top-color: var(--sf-accent, #4fc3f7);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    .loading-msg {
        color: var(--sf-text-primary, #c8e0f4);
        font-size: 12px;
        font-family: var(--sf-font-ui, 'Inter', sans-serif);
        font-weight: 500;
        letter-spacing: 0.03em;
        text-align: center;
        padding: 0 16px;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
</style>
