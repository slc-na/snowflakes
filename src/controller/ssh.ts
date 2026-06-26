import { invoke } from "@tauri-apps/api/core";
import { goto } from "$app/navigation";
import { toast } from "svelte-sonner";
import type { SessionInfo } from "../types/settings";
import { deleteSessionPass, loadSessionPass, saveSessionPass } from "./vault";
import { deleteSessionInfo, loadSessionInfo, saveSessionInfo, loadSettings } from "./local";
import { parseSshTemplate } from "../lib/sshTemplate";

// Matches the Rust-side marker (see ssh_instance.rs bastion_session) used to
// detect a channel that closed right after exec — almost always a wrong
// target host/port rather than a real bastion auth failure.
const FAST_EXIT_MARKER = "FAST_EXIT::";
const FAST_EXIT_TOAST_MESSAGE =
    "The channel was closed immediately, please check your port or contact a netsys";

function handleConnectError(e: unknown): never {
    const message = e instanceof Error ? e.message : String(e);
    if (message.includes(FAST_EXIT_MARKER)) {
        toast.error(FAST_EXIT_TOAST_MESSAGE);
        throw new Error(FAST_EXIT_TOAST_MESSAGE);
    }
    throw e;
}

export async function connectToSession(
    session: SessionInfo,
    onStatus: (status: string) => void
): Promise<void> {
    try {
        onStatus("Fetching credentials...");
        // Re-load to get the password from vault
        console.log(session.bastionIp);
        console.log(session.username);
        console.log(session.targetIp);

        onStatus("Connecting to bastion...");

        const settings = await loadSettings();
        const params = parseSshTemplate(settings.sshTemplate, {
            target_ip: session.targetIp,
            port: session.port ?? 22,
        });

        console.debug("Connecting to bastion with the following parameters:", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            initialPassword: session.password,
            initialUsername: session.username,
            params,
        });

        const res = await invoke("start_ssh_session", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            initialPassword: session.password,
            initialUsername: session.username,
            params,
        });
        const key = res as string;

        onStatus("Updating session history...");
        const updatedInfo: SessionInfo = {
            ...session,
            sessionKey: key,
            connectedAt: Date.now(),
        };
        await saveSessionInfo(key, updatedInfo);
        await saveSessionPass(key, session.password!);
        onStatus("Redirecting...");

        goto(`/session?key=${key}`, {
            state: {
                bastion: session.bastionIp,
                initialUsername: session.username,
                hostname: session.targetIp,
            },
        });
    } catch (e) {
        console.error("[SSH Controller] Connection failed:", e);
        handleConnectError(e);
    }
}


export async function reconnectToSession(
    session: SessionInfo,
    onStatus: (status: string) => void
): Promise<string> {
    try {
        onStatus("Fetching credentials...");

        console.log(session.bastionIp);
        console.log(session.username);
        console.log(session.sessionKey);

        const password = await loadSessionPass(session.sessionKey);

        onStatus("Connecting to bastion...");

        const settings = await loadSettings();
        const params = parseSshTemplate(settings.sshTemplate, {
            target_ip: session.targetIp,
            port: session.port ?? 22,
        });

        const res = await invoke("reconnect_to_session", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            initialPassword: password,
            initialUsername: session.username,
            key: session.sessionKey,
            params,
        });
        const key = session.sessionKey;

        onStatus("Redirecting...");

        return key
    } catch (e) {
        console.error("[SSH Controller] Connection failed:", e);
        handleConnectError(e);
    }
}


export async function deleteSession(session: string): Promise<void> {
    try {
        // await deleteSessionInfo(session);
        // await deleteSessionPass(session);
        await invoke("disconnect", {
            sessionKey: session,
        });
    } catch (e) {
        console.error("[SSH Controller] Delete failed:", e);
        throw e;
    }
}