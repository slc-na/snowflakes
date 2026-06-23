import { invoke } from "@tauri-apps/api/core";
import { goto } from "$app/navigation";
import type { SessionInfo } from "../types/settings";
import { deleteSessionPass, loadSessionPass, saveSessionPass } from "./vault";
import { deleteSessionInfo, loadSessionInfo, saveSessionInfo } from "./local";

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

        console.debug("Connecting to bastion with the following parameters:", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            port: session.port,
            initialPassword: session.password,
            initialUsername: session.username,
        });

        const res = await invoke("start_ssh_session", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            port: session.port ?? 22,
            initialPassword: session.password,
            initialUsername: session.username,
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
        throw e;
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
        const res = await invoke("reconnect_to_session", {
            bastion: session.bastionIp,
            hostname: session.targetIp,
            port: session.port ?? 22,
            initialPassword: password,
            initialUsername: session.username,
            key: session.sessionKey,
        });
        const key = session.sessionKey;

        onStatus("Redirecting...");

        return key
    } catch (e) {
        console.error("[SSH Controller] Connection failed:", e);
        throw e;
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