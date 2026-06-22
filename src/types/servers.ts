export type ServerOs = "linux" | "mac" | "windows";

export type ServerAttribute = {
    description: string;
    id: string;
    ip: string;
    name: string;
    os: ServerOs;
    port: number;
};