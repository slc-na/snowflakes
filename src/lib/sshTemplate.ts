// Parses a user-customizable SSH template (settings.sshTemplate) into the
// key=value parameters sent to the bastion. The "ssh -t {username}@{bastion_ip}"
// prefix is not part of the bastion payload, so any token without "=" is skipped.
// "target" and "port" are always assigned programmatically from the real
// connection values, overriding whatever is written in the template. Any other
// key keeps its literal template value, unless that value still contains an
// unresolved "{placeholder}" — those keys are dropped since nothing assigns them.
export function parseSshTemplate(
    template: string,
    vars: { target_ip: string; port: number | string }
): Record<string, string> {
    const params: Record<string, string> = {};
    const tokens = template.trim().split(/\s+/);

    for (const token of tokens) {
        const eqIndex = token.indexOf("=");
        if (eqIndex === -1) continue;

        const key = token.slice(0, eqIndex);
        const rawValue = token.slice(eqIndex + 1);

        if (key === "target") {
            params[key] = String(vars.target_ip);
        } else if (key === "port") {
            params[key] = String(vars.port);
        } else if (/\{[^}]+\}/.test(rawValue)) {
            continue;
        } else {
            params[key] = rawValue;
        }
    }

    return params;
}
