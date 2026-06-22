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

    let currentSshKey = $state<string | null>(null);
    let serializeAddon = $state<SerializeAddon | null>(null);
        
    let term : Terminal = $state(new Terminal());
    let fitAddon : FitAddon;
    let cleanupSsh : () => void;


    function handleResize() {
        fitAddon.fit()
    };


    // ini handle key event yg diluar terminal
    function handleKeyDown(e: KeyboardEvent){

        // Esc lepasin focus dari terminal, supaya Ctrl+Tab / Ctrl+Shift+Tab / Ctrl+T
        // (handled di window-level listener di SessionTabBar) bisa dipakai pindah/keluar sesi
        if (e.key === "Escape") {
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


        // handle ctrl + alt +  c = copy
        // if (e.ctrlKey && e.altKey && e.key.toLowerCase() === 'c') {
        //     if (e.type === 'keydown') {
        //         const selection = term.getSelection();
        //         if (selection) {
        //             navigator.clipboard.writeText(selection);
        //         }
        //     }
        //     e.preventDefault();
        //     return false; // stop xterm from processing this further
        // }

        // Ctrl+Alt+V — Paste
        // if (e.ctrlKey && e.altKey && e.key.toLowerCase() === 'v') {
        //     if (e.type === 'keydown') {
        //         navigator.clipboard.readText().then((text) => {
        //             term.paste(text);
        //         }).catch((err) => {
        //             console.error("Clipboard read failed:", err);
        //         });
        //     }
        //     e.preventDefault();
        //     return false;
        // }

        return true; // let xterm handle everything else normally
    };


    function handleScroll(e : WheelEvent){
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
        

        serializeAddon = new SerializeAddon();
        term.loadAddon(serializeAddon);

        term.open(terminalElement);
        fitAddon.fit();
        term.focus();
    }


    async function setupSsh(key : string){

        console.debug(`Setup SSH called for key : ${key}`)


        // cleanup ssh connection yang tab sebelumnya dlu
        // klo gaada ini, connectionnya ttp persists ketika user pindah tab
        if(cleanupSsh){
            console.debug("Cleaning up ssh")
            cleanupSsh()
        }

        const currentSession = await loadSessionInfo(key);
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
        unlisten = await listen(eventName, (event) => {

            // console.debug("user typed : " + event.payload)

            term.write(event.payload as string);
        });

        const errorEventName = `ssh-error-output-${key}`;

        unlistenError = await listen(errorEventName, (event) => {
            isOpenerrorMessage = true;
            errorMessage = event.payload as string;
        });

        let inputBuffer = "";
        let debounceTimer: ReturnType<typeof setTimeout> | null = null;



    
        let detachTermOnData = term.onData((data: string) => {
            inputBuffer += data;

            if (debounceTimer) return;

            debounceTimer = setTimeout(() => {
                const payload = inputBuffer;
                inputBuffer = "";
                debounceTimer = null;

                console.debug(`Sending string : ${payload}`)

                invoke("send_ssh_input", {
                    input: payload,
                    ip: key,
                });
            }, 70);
        });


        // TODO : pick antara mau by selection atau by ctrl + alt + c
        // let detachTermOnSelection =  term.onSelectionChange(() => {
        //     const selection = term.getSelection();
        //     if (selection) {
        //         navigator.clipboard.writeText(selection);
        //     }
        // })



        term.attachCustomKeyEventHandler(handleKeyDown);

        cleanupSsh = function(){
            console.debug(`Session : ${session?.sessionKey} : SSH cleaned up`)
            detachTermOnData.dispose(); //lepasin onData
            // detachTermOnSelection.dispose() //lepasin onSelect
            if (debounceTimer) clearTimeout(debounceTimer);
        };

    }



    function teardownSsh(){
        if (
            currentSshKey &&
            currentSshKey !== "unknown" &&
            serializeAddon
        ) {

            console.debug(`Teardown ssh called on ${currentSshKey}`)
            saveTerminalState(currentSshKey, serializeAddon.serialize());
        }
        if (unlisten) unlisten();
        if (unlistenError) unlistenError();
    }

    function setupWindowEventListeners(){
        

        window.addEventListener("resize", handleResize);
        window.addEventListener("wheel", handleScroll);
    }

    function removeWindowEventListeners(){
        window.addEventListener("resize", handleResize);
        window.addEventListener("wheel", handleScroll);
    }

    
    
    // tiap targetKey berubah, ini jalan
    // TODO : ketika user pindah tab, pastiin connectionnnya mati tp terminalnya ga diclear
     $effect(() => {
        console.debug(`Side effect triggered with targetKey : ${targetKey}, currentKy : ${currentSshKey}`)
        
        
        // klo newKey (yg di params) beda dgn yg current, init koneksi baru
        if (targetKey !== currentSshKey) {

            if(currentSshKey && serializeAddon){
                // apabila skrg uda buka tab, lalu mau pindah tab, kita savve dlu state skrg
                saveTerminalState(currentSshKey, serializeAddon.serialize())
            }

            if (targetKey && targetKey !== "unknown") {
                console.debug("SSH Key changed or initialized:", targetKey);

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


    // mount cuma jalan ketika first dtg ke page, klo pindah tab 1 ke tab 2 (navigasi yang pake search params), onMountnya ga jalan, yg jalan yang $effect di atas
    onMount(() => {
        setupTerminal()
        setupWindowEventListeners();
    });


    
    onDestroy(() => {
        console.debug(`ID : ${session?.sessionKey}, page destroyed (OnDestroy)`)
        teardownSsh()
        removeWindowEventListeners();
        term.dispose();
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
