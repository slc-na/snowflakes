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

// Tauri window labels only allow alphanumeric/-/:/_, which sessionKey already
// satisfies (ip-with-dashes + uuid), but prefix it so it can't collide with
// any other window label in the app.
export function guacamoleWindowLabel(sessionKey: string): string {
    return `guac-${sessionKey}`;
}

// Opens (or focuses, if already open) the dedicated webview window that hosts
// the actual Guacamole remote desktop for this session. Iframe-based token
// injection is blocked by the browser's same-origin policy, so the real
// connection lives in its own Tauri window instead - see open_guacamole_window
// in src-tauri/src/http/guacamole.rs.
export async function openGuacamoleWindow(sessionKey: string): Promise<void> {
    const token = getGuacamoleToken(sessionKey);
    if (!token) {
        throw new Error("Guacamole token not found for this session - reconnect from the servers page.");
    }
    await invoke("open_guacamole_window", {
        label: guacamoleWindowLabel(sessionKey),
        token,
    });
}

export async function focusGuacamoleWindow(sessionKey: string): Promise<boolean> {
    return invoke<boolean>("focus_guacamole_window", {
        label: guacamoleWindowLabel(sessionKey),
    });
}

async function closeGuacamoleWindow(sessionKey: string): Promise<void> {
    await invoke("close_guacamole_window", {
        label: guacamoleWindowLabel(sessionKey),
    });
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

    onStatus("Opening remote desktop window...");
    await openGuacamoleWindow(key);

    onStatus("Guacamole session ready.");
    return key;
}

export async function disconnectGuacamoleSession(sessionKey: string): Promise<void> {
    setActiveGuacamoleSessions(
        getActiveGuacamoleSessions().filter((key) => key !== sessionKey),
    );
    sessionStorage.removeItem(`${TOKEN_KEY_PREFIX}${sessionKey}`);
    window.dispatchEvent(new CustomEvent(GUACAMOLE_SESSION_UPDATED_EVENT));
    await closeGuacamoleWindow(sessionKey);
}
