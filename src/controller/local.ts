import type { ServerAttribute, ServerOs } from "../types/servers";
import { DEFAULT_SETTINGS, type SessionInfo, type SnowflakesSettings } from "../types/settings";
import { invoke } from '@tauri-apps/api/core';

const SETTINGS_KEY = "snowflakes_settings";
const SERVERS_CACHE_KEY = "snowflakes_servers_cache";
const SERVER_OVERRIDES_KEY = "snowflakes_server_overrides";



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

type ServerOverride = { os?: ServerOs; ssh_port?: number };

function loadServerOverrides(): Record<string, ServerOverride> {
    const raw = localStorage.getItem(SERVER_OVERRIDES_KEY);
    if (!raw) return {};
    try {
        return JSON.parse(raw);
    } catch {
        return {};
    }
}

export function saveServerOverride(serverId: string, override: ServerOverride): void {
    const overrides = loadServerOverrides();
    overrides[serverId] = { ...overrides[serverId], ...override };
    localStorage.setItem(SERVER_OVERRIDES_KEY, JSON.stringify(overrides));
}

function applyServerOverrides(servers: ServerAttribute[]): ServerAttribute[] {
    const overrides = loadServerOverrides();
    return servers.map((server) => ({
        ...server,
        ...overrides[server.id],
    }));
}

export function getCachedServers(): ServerAttribute[] | null {
    const raw = localStorage.getItem(SERVERS_CACHE_KEY);
    if (!raw) return null;
    try {
        return applyServerOverrides(JSON.parse(raw));
    } catch {
        return null;
    }
}

function cacheServers(servers: ServerAttribute[]): void {
    localStorage.setItem(SERVERS_CACHE_KEY, JSON.stringify(servers));
}

export async function getAllServers(): Promise<ServerAttribute[]> {
    const raw: any = await invoke("get_server");
    const data = raw.data;
    const servers: ServerAttribute[] = data.map((item: any) => ({
        id: item.id,
        name: item.name,
        ip: item.ip,
        description: item.description,
        os: item.os as ServerOs,
        ssh_port: item.ssh_port,
    }));
    cacheServers(servers);
    return applyServerOverrides(servers);
}

export async function updateServer(server: ServerAttribute): Promise<void> {
    saveServerOverride(server.id, { os: server.os, ssh_port: server.ssh_port });
    await invoke("update_server", {
        serverId: String(server.id),
        params: {
            name: server.name,
            ip: server.ip,
            description: server.description,
            os : server.os,
            ssh_port: server.ssh_port,
        },
    });
}