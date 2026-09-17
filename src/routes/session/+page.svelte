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

    let unlisten: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let isOpenerrorMessage = $state(false);
    let errorMessage = $state("");

    let currentSshKey = $state<string | null>(null);
    let serializeAddon = $state<SerializeAddon | null>(null);

    let term : Terminal = $state(new Terminal());
    let fitAddon : FitAddon;
    let cleanupSsh : (() => void) | undefined;
    let resizeObserver: ResizeObserver;

    // Monotonic token identifying the "current" tab setup. Any async work started
    // for an older token is stale and must not write to / attach listeners on the
    // shared terminal of the newly selected tab.
    let sessionGeneration = 0;


    // xterm parses writes asynchronously (chunked across several event-loop ticks).
    // `clear()`/`reset()` do NOT cancel data that is already queued, so the tail of
    // the previous tab's output/scrollback-restore would otherwise keep rendering
    // after the terminal has been handed to the next tab. Writing an empty chunk
    // with a callback queues a barrier that fires once everything pending is parsed.
    function drainTerminalWrites(target: Terminal): Promise<void> {
        return new Promise((resolve) => target.write("", () => resolve()));
    }


    function handleResize() {
        fitAddon.fit()
    };


    // ini handle key event yg diluar terminal
    function handleKeyDown(e: KeyboardEvent){

        // Esc lepasin focus dari terminal, supaya Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+T
        // (handled di window-level listener di SessionTabBar) bisa dipakai pindah/keluar sesi
        // Re-attach dihandle di handleGlobalEscape (window-level), bukan disini,
        // karena handler ini cuma kepanggil pas terminal lagi focus.
        // e.type dicek "keydown" doang - xterm panggil handler ini juga pas keyup,
        // dan tanpa guard ini, keyup dari Escape yang sama yang baru aja dipake
        // buat refocus (di handleGlobalEscape) bakal langsung re-blur lagi.
        if (e.key === "Escape" && e.type === "keydown") {
            e.preventDefault();
            term.blur();
            return false;
        }

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


        

        return true; // let xterm handle everything else normally
    };


    function setupTerminal(){
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


        console.log(targetKey);

        fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        
        loadSettings().then(setting=>{
            setTerminalFont(term!, fitAddon, setting.fontSize)  
        })

        resizeObserver = new ResizeObserver(() => {
            // rAF avoids "ResizeObserver loop limit exceeded" and ensures layout has settled
            requestAnimationFrame(() => fitAddon.fit());
        });
        resizeObserver.observe(terminalElement);

        serializeAddon = new SerializeAddon();
        term.loadAddon(serializeAddon);

        term.open(terminalElement);
        fitAddon.fit();
        term.focus();
    }


    // Dispose listeners/callbacks belonging to the currently attached session.
    // Always called before switching tabs so a stale `ssh-output` listener can
    // never write the previous tab's output into the newly selected tab.
    function disposeActiveSession(){
        if (unlisten) {
            unlisten();
            unlisten = undefined;
        }
        if (unlistenError) {
            unlistenError();
            unlistenError = undefined;
        }
        if (cleanupSsh) {
            cleanupSsh();
            cleanupSsh = undefined;
        }
    }


    async function setupSsh(key : string, generation : number){

        const isCurrent = () =>
            generation === sessionGeneration && key === currentSshKey;

        console.debug(`Setup SSH called for key : ${key} (generation ${generation})`)

        const currentSession = await loadSessionInfo(key);
        if (!isCurrent()) {
            console.debug(`SSH setup for ${key} aborted (stale generation)`)
            return;
        }
        if (!currentSession) {
            term.writeln(
                `\r\n\x1b[31mError: Session info not found for key: ${key}\x1b[0m`,
            );
            return;
        }
        session = currentSession;

        const eventName = `ssh-output-${key}`;

        // klo idnya uda ada history, kita load aja
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
            term.writeln(` Time         : ${new Date().toLocaleString()}`);
            term.writeln("----------------------------------------");
            term.writeln(` Press Enter to continue`);
            term.writeln("========================================");
            term.writeln("");
        }


        // listen to tauri emitted event
        const localUnlisten = await listen(eventName, (event) => {
            // a listener left over from a previously selected tab must never
            // write into the terminal that currently shows another session
            if (!isCurrent()) return;

            // console.debug("user typed : " + event.payload)

            term.write(event.payload as string);
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
        // nano know the real viewport height and can scroll long documents
        const detachTermOnResize = term.onResize(({ cols, rows }) => {
            if (!isCurrent()) return;
            invoke("resize_ssh_pty", { cols, rows, ip: key }).catch((e) =>
                console.debug("resize_ssh_pty failed", e),
            );
        });

        // the pty was created with the host terminal size (or a fallback)
        // before this page mounted, so push the real xterm size once here
        if (term.cols > 0 && term.rows > 0) {
            invoke("resize_ssh_pty", {
                cols: term.cols,
                rows: term.rows,
                ip: key,
            }).catch((e) => console.debug("resize_ssh_pty failed", e));
        }

        let inputBuffer = "";
        let debounceTimer: ReturnType<typeof setTimeout> | null = null;

        const detachTermOnData = term.onData((data: string) => {
            if (!isCurrent()) return;

            inputBuffer += data;

            if (debounceTimer) return;

            debounceTimer = setTimeout(() => {
                const payload = inputBuffer;
                inputBuffer = "";
                debounceTimer = null;

                if (!isCurrent()) return;

                console.debug(`Sending string : ${payload}`)

                invoke("send_ssh_input", {
                    input: payload,
                    ip: key,
                });
            }, 70);
        });


        // copy selection by mouse selection (kyk putty)
        const detachTermOnSelection =  term.onSelectionChange(() => {
            if (!isCurrent()) return;
            const selection = term.getSelection();
            if (selection) {
                navigator.clipboard.writeText(selection);
            }
        })


        term.attachCustomKeyEventHandler(handleKeyDown);

        cleanupSsh = function(){
            console.debug(`Session : ${key} : SSH cleaned up`)
            detachTermOnData.dispose(); //lepasin onData
            detachTermOnSelection.dispose() //lepasin onSelect
            detachTermOnResize.dispose() //lepasin onResize
            if (debounceTimer) clearTimeout(debounceTimer);
        };

    }



    function teardownSsh(){
        // cancel any in-flight setupSsh before tearing the page down
        sessionGeneration += 1;

        if (
            currentSshKey &&
            currentSshKey !== "unknown" &&
            serializeAddon
        ) {

            console.debug(`Teardown ssh called on ${currentSshKey}`)
            saveTerminalState(currentSshKey, serializeAddon.serialize());
        }
        disposeActiveSession();
    }

    // klo escape kepencet pas terminal lagi gak focus (uda di-blur sebelumnya),
    // toggle balik focusnya ke terminal. e.defaultPrevented dicek supaya gak
    // langsung re-focus di event yg sama yg baru aja dipake buat blur (handleKeyDown
    // udah preventDefault duluan kalo terminal masih focus pas Escape ditekan).
    function handleGlobalEscape(e: KeyboardEvent){
        if (e.key !== "Escape" || e.defaultPrevented) return;
        e.preventDefault();
        
        term.focus();
    }

    function setupWindowEventListeners(){


        window.addEventListener("resize", handleResize);
        window.addEventListener("keydown", handleGlobalEscape);
    }

    function removeWindowEventListeners(){
        window.removeEventListener("resize", handleResize);
        window.removeEventListener("keydown", handleGlobalEscape);
    }

    
    
    // tiap targetKey berubah, ini jalan
    // TODO : ketika user pindah tab, pastiin connectionnnya mati tp terminalnya ga diclear
     $effect(() => {
        console.debug(`Side effect triggered with targetKey : ${targetKey}, currentKy : ${currentSshKey}`)
        
        
        // klo newKey (yg di params) beda dgn yg current, init koneksi baru
        if (targetKey !== currentSshKey) {

            // invalidate any in-flight setupSsh from the previous tab first, so
            // its async continuation can't write into the terminal after switch
            sessionGeneration += 1;
            const generation = sessionGeneration;
            const outgoingKey = currentSshKey;

            // stop the outgoing session from queueing new output before we drain
            disposeActiveSession();

            // Only after xterm has parsed every byte already queued is it safe to
            // reuse the terminal: without this, the tail of the previous tab's
            // output/restore renders into the newly selected tab.
            void drainTerminalWrites(term).then(() => {
                if (generation !== sessionGeneration) return;

                if (outgoingKey && outgoingKey !== "unknown" && serializeAddon) {
                    // apabila skrg uda buka tab, lalu mau pindah tab, kita savve dlu state skrg
                    saveTerminalState(outgoingKey, serializeAddon.serialize())
                }

                if (targetKey && targetKey !== "unknown") {
                    console.debug("SSH Key changed or initialized:", targetKey);

                    // reset (not clear) so buffered lines, scrollback and terminal
                    // modes set by the previous session can't leak into this one
                    term.reset();
                    currentSshKey = targetKey;
                    setupSsh(targetKey, generation);
                } else {
                    currentSshKey = "unknown";
                }
            });
        }
    });


    // mount cuma jalan ketika first dtg ke page, klo pindah tab 1 ke tab 2 (navigasi yang pake search params), onMountnya ga jalan, yg jalan yang $effect di atas
    onMount(() => {
        setupTerminal()
        setupWindowEventListeners();
    });


    
    onDestroy(() => {
        console.debug(`ID : ${session?.sessionKey}, page destroyed (OnDestroy)`)
        teardownSsh()
        removeWindowEventListeners();
        resizeObserver?.disconnect();
        term.dispose();
    });



</script>

<div class="flex flex-col h-full bg-[#1a1b26]">
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

    <div class="flex-1 min-h-0 p-2">
         <div class="h-full w-full pb-10 min-h-0">
        <!-- inner: this is what xterm actually measures and renders into -->
            <div
                bind:this={terminalElement}
                class="h-full w-full"
            >
            </div>
        </div>
    </div>
    <ErrorModal
        isOpen={isOpenerrorMessage}
        {errorMessage}
        sessionKey={currentSshKey}
    />
</div>

<style>
</style>
