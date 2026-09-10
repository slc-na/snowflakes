export type SnowflakesSettings = {
    bastionIp: string;
    sshTemplate: string;
    fontSize: number;
};

export type SessionInfo = {
    sessionKey: string;
    username: string;
    password?: string;
    targetIp: string;
    port?: number;
    bastionIp: string;
    label: string;
    connectedAt: number;
};

export type DefaultAccount = {
    username: string;
    password: string;
};

export const DEFAULT_SETTINGS: SnowflakesSettings = {
    bastionIp: "10.22.77.248",
    // target = target IP
    // type = "ssh" or "exec"
    // exec = custom bash script
    sshTemplate: "ssh -t {username}@{bastion_ip} target={target} port={port}",
    fontSize: 13
};
