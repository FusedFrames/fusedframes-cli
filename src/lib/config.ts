import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

interface Config {
  apiKey?: string;
}

function getConfigDir(): string {
  return join(homedir(), ".config", "fusedframes");
}

function getConfigPath(): string {
  return join(getConfigDir(), "config.json");
}

export function readConfig(): Config {
  try {
    const data = readFileSync(getConfigPath(), "utf-8");
    return JSON.parse(data) as Config;
  } catch {
    return {};
  }
}

export function writeConfig(config: Config): void {
  const dir = getConfigDir();
  mkdirSync(dir, { recursive: true, mode: 0o700 });

  const path = getConfigPath();
  writeFileSync(path, JSON.stringify(config, null, 2) + "\n", {
    mode: 0o600,
  });

  // `writeFileSync`'s `mode` only applies when the file is newly created, so a
  // config.json that already existed with looser permissions (user-created, or
  // restored from a backup that dropped perms) would never be tightened. Force
  // owner-only permissions on every write so the plaintext key can't sit
  // world/group-readable.
  try {
    chmodSync(dir, 0o700);
    chmodSync(path, 0o600);
  } catch {
    // chmod is unsupported on some filesystems (e.g. Windows) — the key is
    // still written; best-effort hardening only.
  }
}

function getApiKey(): string | undefined {
  // Env var takes precedence
  if (process.env.FUSEDFRAMES_API_KEY) {
    return process.env.FUSEDFRAMES_API_KEY;
  }
  return readConfig().apiKey;
}

export function getApiUrl(): string {
  return process.env.FUSEDFRAMES_API_URL || "https://api.fusedframes.com";
}

export function requireApiKey(): string {
  const key = getApiKey();
  if (!key) {
    throw new Error(
      'API key not configured. Run: echo "ff_..." | fusedframes config set-key, or set the FUSEDFRAMES_API_KEY environment variable.'
    );
  }
  return key;
}

export function getConfigInfo(): {
  apiKey: string | null;
  apiKeySource: string;
  apiUrl: string;
  apiUrlSource: string;
  configPath: string;
} {
  const envKey = process.env.FUSEDFRAMES_API_KEY;
  const fileConfig = readConfig();

  return {
    apiKey: envKey
      ? `${envKey.slice(0, 8)}...`
      : fileConfig.apiKey
        ? `${fileConfig.apiKey.slice(0, 8)}...`
        : null,
    apiKeySource: envKey ? "environment" : fileConfig.apiKey ? "config" : "none",
    apiUrl: getApiUrl(),
    apiUrlSource: process.env.FUSEDFRAMES_API_URL ? "environment" : "default",
    configPath: getConfigPath(),
  };
}
