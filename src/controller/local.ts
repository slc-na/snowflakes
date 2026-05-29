import type { ServerAttribute } from "../types/servers";
import { DEFAULT_SETTINGS, type SessionInfo, type SnowflakesSettings } from "../types/settings";
import { invoke } from '@tauri-apps/api/core';

const SETTINGS_KEY = "snowflakes_settings";



export function saveSettings(settings: SnowflakesSettings): void {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export async function loadSettings(): Promise<SnowflakesSettings> {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    try {
        return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    } catch {
        return { ...DEFAULT_SETTINGS };
    }
}

export async function saveSessionInfo(sessionKey: string, sessionInfo: SessionInfo): Promise<void> {
    sessionInfo.password = ""
    localStorage.setItem(`session:${sessionKey}`, JSON.stringify(sessionInfo));
}

export async function loadSessionInfo(sessionKey: string): Promise<SessionInfo | null> {
    const raw = localStorage.getItem(`session:${sessionKey}`);
    if (!raw) return null;
    try {
        return JSON.parse(raw);
    } catch {
        return null;
    }
}

export async function deleteSessionInfo(sessionKey: string): Promise<void> {
    localStorage.removeItem(`session:${sessionKey}`);
}

export async function loadAllSessions(): Promise<SessionInfo[]> {
    const sessions: SessionInfo[] = [];
    for (const key in localStorage) {
        if (key.startsWith("session:")) {
            const session = JSON.parse(localStorage.getItem(key)!);
            sessions.push(session);
        }
    }
    return sessions;
}

export async function getAllServers() : Promise<ServerAttribute[]>{
    const raw: any = await invoke("get_server");
    const data = raw.data;
    const servers: ServerAttribute[] = data.map((item: any) => ({
        id: item.id,
        name: item.name,
        ip: item.ip,
        description: item.description,
    }));
    return servers;
}