<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import { page } from "$app/stores"; // SvelteKit store
    import { Terminal } from "@xterm/xterm";
    import { FitAddon } from "@xterm/addon-fit";
    import { SerializeAddon } from "@xterm/addon-serialize";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { invoke } from "@tauri-apps/api/core";
    import "@xterm/xterm/css/xterm.css";
    import ErrorModal from "../../components/modal/ErrorModal.svelte";
    import type { SessionInfo } from "../../types/settings";
    import { loadSessionInfo } from "../../controller/local";
    import { deleteSession } from "../../controller/ssh";
    import {
        saveTerminalState,
        loadTerminalState,
    } from "../../controller/session";
    import { goto } from "$app/navigation";
    import { incrementTerminalFont, setTerminalFont } from "../../lib/resizeTerminal";
    
    import {
        loadSettings,
    } from "../../controller/local";
    // Ambil IP dari query param ?ip=...
    let targetKey = $derived($page.url.searchParams.get("key") || "unknown");
    let session = $state<SessionInfo | null>(null);
    let terminalElement: HTMLElement;

    let unlisten: UnlistenFn;
    let unlistenError: UnlistenFn;
    let isOpenerrorMessage = $state(false);
    let errorMessage = $state("");
    let heldKeys = $state([])

    let currentSshKey = $state<string | null>(null);
    let serializeAddon = $state<SerializeAddon | null>(null);

    onMount(() => {

        const term = new Terminal({
            theme: {
                background: "#1a1b26",
                foreground: "#a9b1d6",
                cursor: "#f7768e",
            },
            cursorBlink: true,
            fontFamily: "JetBrains Mono, monospace",
            fontSize: 13,
        });


        console.log(targetKey);
        const now = new Date().toLocaleString();

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        
        loadSettings().then(setting=>{
          setTerminalFont(term, fitAddon, setting.fontSize)  
        })
        

        serializeAddon = new SerializeAddon();
        term.loadAddon(serializeAddon);

        term.open(terminalElement);
        fitAddon.fit();
        term.focus();

        const setupSsh = async (key: string) => {
            const currentSession = await loadSessionInfo(key);
            if (!currentSession) {
                term.writeln(
                    `\r\n\x1b[31mError: Session info not found for key: ${key}\x1b[0m`,
                );
                return;
            }
            session = currentSession;

            const eventName = `ssh-output-${key}`;

            const previousState = loadTerminalState(key);
            if (previousState) {
                term.write(previousState);
            } else {
                term.writeln("");
                term.writeln("========================================");
                term.writeln("          SSH CONNECTION INFO          ");
                term.writeln("----------------------------------------");
                term.writeln(` Session Key  : ${key}`);
                term.writeln(` Bastion Host : ${currentSession.bastionIp}`);
                term.writeln(` Username     : ${currentSession.username}`);
                term.writeln(` Target Host  : ${currentSession.targetIp}`);
                term.writeln(` Time         : ${now}`);
                term.writeln("----------------------------------------");
                term.writeln(` Press Enter to continue`);
                term.writeln("========================================");
                term.writeln("");
            }

            unlisten = await listen(eventName, (event) => {
                term.write(event.payload as string);
            });

            const errorEventName = `ssh-error-output-${key}`;

            unlistenError = await listen(errorEventName, (event) => {
                isOpenerrorMessage = true;
                errorMessage = event.payload as string;
            });

            let inputBuffer = "";
            let debounceTimer: ReturnType<typeof setTimeout> | null = null;

            term.onData((data: string) => {
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

        const handleResize = () => fitAddon.fit();

        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.ctrlKey) {
                if (e.key === "=" || e.key === "+") {
                    e.preventDefault();
                    incrementTerminalFont(term, fitAddon, 1)
                } else if (e.key === "-") {
                    e.preventDefault();
                    incrementTerminalFont(term, fitAddon, -1)
                } else if (e.key === "0") {
                    e.preventDefault();
                    term.options.fontSize = 13;
                    fitAddon.fit();
                }
            }
        };
        
        const handleScroll = (e : WheelEvent) => {
            if(!e.ctrlKey){
                return
            }
            
            // if scroll up > resize up
            e.preventDefault();
            let increment = 0;

            if(e.deltaY > 0){
                increment = -1
            }else{
                increment = 1
            }

            incrementTerminalFont(term, fitAddon, increment)
            // if scroll down > resize down
        }

        $effect(() => {
            if (targetKey !== currentSshKey) {
                if (targetKey && targetKey !== "unknown") {
                    console.log("SSH Key changed or initialized:", targetKey);

                    // cleanup listener lama
                    if (unlisten) unlisten();
                    if (unlistenError) unlistenError();

                    term.clear();
                    currentSshKey = targetKey;
                    setupSsh(targetKey);
                } else {
                    currentSshKey = "unknown";
                }
            }
        });


        window.addEventListener("resize", handleResize);
        window.addEventListener("keydown", handleKeyDown);
        window.addEventListener("wheel", handleScroll);

        return () => {
            if (
                currentSshKey &&
                currentSshKey !== "unknown" &&
                serializeAddon
            ) {
                saveTerminalState(currentSshKey, serializeAddon.serialize());
            }
            window.removeEventListener("resize", handleResize);
            window.removeEventListener("keydown", handleKeyDown);
            if (unlisten) unlisten();
            if (unlistenError) unlistenError();
            term.dispose();
        };
    });

    onDestroy(() => {
        if (currentSshKey && currentSshKey !== "unknown" && serializeAddon) {
            saveTerminalState(currentSshKey, serializeAddon.serialize());
        }
    });
</script>

<div class="flex flex-col h-screen bg-[#1a1b26]">
    <div
        class="px-4 py-2 bg-[#16161e] border-b border-[#24283b] flex justify-between items-center"
    >
        <div class="text-[11px] text-[#565f89] font-mono">
            CONNECTED TO: <span class="text-[#7aa2f7]"
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
            onclick={async () => {
                await deleteSession(targetKey);
                goto("/");
            }}
        >
            Disconnect
        </button>
    </div>

    <div class="flex-1 p-2">
        <div
            bind:this={terminalElement}
            class="h-[calc(100%-32px)] w-full"
        ></div>
    </div>
    <ErrorModal
        isOpen={isOpenerrorMessage}
        {errorMessage}
        sessionKey={currentSshKey}
    />
</div>

<style>
</style>
