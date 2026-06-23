import { invoke } from "@tauri-apps/api/core";
import type { ServerAttribute } from "../types/servers";
import type { SessionInfo } from "../types/settings";
import { loadDefaultAccount } from "./vault";
import { saveSessionInfo } from "./local";

const ACTIVE_SESSIONS_KEY = "snowflakes_active_guac_sessions";
const TOKEN_KEY_PREFIX = "guac_token_";
export const GUACAMOLE_SESSION_UPDATED_EVENT = "guacamole_session_updated";

export function getActiveGuacamoleSessions(): string[] {
    const raw = localStorage.getItem(ACTIVE_SESSIONS_KEY);
    if (!raw) return [];
    try {
        return JSON.parse(raw);
    } catch {
        return [];
    }
}

function setActiveGuacamoleSessions(keys: string[]): void {
    localStorage.setItem(ACTIVE_SESSIONS_KEY, JSON.stringify(keys));
}

export function getGuacamoleToken(sessionKey: string): string | null {
    return sessionStorage.getItem(`${TOKEN_KEY_PREFIX}${sessionKey}`);
}

export async function connectToGuacamoleSession(
    server: ServerAttribute,
    onStatus: (status: string) => void,
): Promise<string> {
    onStatus("Fetching credentials...");
    const account = await loadDefaultAccount();
    if (!account.username) {
        throw new Error("Set a Default Account in Settings before using Guacamole Quick Connect.");
    }

    onStatus("Authenticating with Guacamole...");
    const authToken = await invoke<string>("guacamole_login", {
        username: account.username,
        password: account.password,
    });

    const key = `${server.ip.replace(/\./g, "-")}_${crypto.randomUUID()}`;
    sessionStorage.setItem(`${TOKEN_KEY_PREFIX}${key}`, authToken);

    const sessionInfo: SessionInfo = {
        sessionKey: key,
        username: account.username,
        targetIp: server.ip,
        bastionIp: "",
        label: server.name,
        connectedAt: Date.now(),
    };
    await saveSessionInfo(key, sessionInfo);

    setActiveGuacamoleSessions([...getActiveGuacamoleSessions(), key]);
    window.dispatchEvent(new CustomEvent(GUACAMOLE_SESSION_UPDATED_EVENT));

    onStatus("Guacamole session ready.");
    return key;
}

export function disconnectGuacamoleSession(sessionKey: string): void {
    setActiveGuacamoleSessions(
        getActiveGuacamoleSessions().filter((key) => key !== sessionKey),
    );
    sessionStorage.removeItem(`${TOKEN_KEY_PREFIX}${sessionKey}`);
    window.dispatchEvent(new CustomEvent(GUACAMOLE_SESSION_UPDATED_EVENT));
}
