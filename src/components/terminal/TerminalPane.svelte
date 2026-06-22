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
    import { incrementTerminalFont } from "$lib/resizeTerminal";

    let { targetKey, onClose } = $props<{
        targetKey: string;
        onClose: () => void;
    }>();
    let connectingStatus = $state("");
    let isLoading = $state(false);
    let session = $state<SessionInfo | null>(null);
    let terminalElement: HTMLElement;

    let unlisten: UnlistenFn;
    let unlistenError: UnlistenFn;
    let isOpenerrorMessage = $state(false);
    let errorMessage = $state("");

    let currentSshKey = $state<string | null>(null);
    let serializeAddon = $state<SerializeAddon | null>(null);
    let term: Terminal | null = null;
    let fitAddon: FitAddon | null = null;

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

        const setupSsh = async (key: string) => {
            isLoading = true;
            connectingStatus = "Initializing connection...";
            const currentSession = await loadSessionInfo(key);
            if (!currentSession) {
                term?.writeln(
                    `\r\n\x1b[31mError: Session info not found for key: ${key}\x1b[0m`,
                );
                isLoading = false;
                return;
            }
            session = currentSession;

            const eventName = `ssh-output-${key}`;

            await reconnectToSession(session, (msg) => {
                connectingStatus = msg;
            });
            
            isLoading = false;
            
            const previousState = loadTerminalState(key);
            if (previousState) {
                term?.write(previousState);
            } else {
                term?.writeln("");
                term?.writeln("========================================");
                term?.writeln("          SSH CONNECTION INFO          ");
                term?.writeln("----------------------------------------");
                term?.writeln(` Session Key  : ${key}`);
                term?.writeln(` Bastion Host : ${currentSession.bastionIp}`);
                term?.writeln(` Username     : ${currentSession.username}`);
                term?.writeln(` Target Host  : ${currentSession.targetIp}`);
                term?.writeln(` Time         : ${now}`);
                term?.writeln("----------------------------------------");
                term?.writeln(` Press Enter to continue`);
                term?.writeln("========================================");
                term?.writeln("");
            }

            unlisten = await listen(eventName, (event) => {
                term?.write(event.payload as string);
            });

            const errorEventName = `ssh-error-output-${key}`;

            unlistenError = await listen(errorEventName, (event) => {
                isOpenerrorMessage = true;
                errorMessage = event.payload as string;
            });

            let inputBuffer = "";
            let debounceTimer: ReturnType<typeof setTimeout> | null = null;

            term?.onData((data: string) => {
                inputBuffer += data;

                if (debounceTimer) return;

                debounceTimer = setTimeout(() => {
                    const payload = inputBuffer;
                    inputBuffer = "";
                    debounceTimer = null;

                    invoke("send_ssh_input", {
                        input: payload,
                        ip: key,
                    });
                }, 70);
            });
        };

        const handleResize = () => {
            setTimeout(() => {
                if (fitAddon) fitAddon.fit();
            }, 50);
        };

        const customKeyHandler = (e: KeyboardEvent) => {

            if (e.key === "Escape" && term) {
                e.preventDefault();
                term.blur();
                return false;
            }

            let increment = 0;

            if (e.type === 'keydown' && e.ctrlKey && term && fitAddon) {
                if (e.key === "=" || e.key === "+") {
                    e.preventDefault();
                    increment = 1;
                } else if (e.key === "-") {
                    e.preventDefault();
                    increment = -1;
                }

                incrementTerminalFont(term, fitAddon, increment)
                return false
            }
            return true;
        };
        term.attachCustomKeyEventHandler(customKeyHandler);

        // TODO : tambahin control + scroll

        $effect(() => {
            if (targetKey !== currentSshKey) {
                if (targetKey && targetKey !== "unknown") {
                    if (unlisten) unlisten();
                    if (unlistenError) unlistenError();

                    term?.clear();
                    currentSshKey = targetKey;
                    setupSsh(targetKey);
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
            window.removeEventListener("resize", handleResize);
            resizeObserver.disconnect();
            if (unlisten) unlisten();
            if (unlistenError) unlistenError();
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
