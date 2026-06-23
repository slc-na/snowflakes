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
    bastionIp: string;
    label: string;
    connectedAt: number;
};

export type DefaultAccount = {
    username: string;
    password: string;
};

export const DEFAULT_SETTINGS: SnowflakesSettings = {
    bastionIp: "",
    sshTemplate: "ssh -t {username}@{bastion} {target}",
    fontSize: 13
};
