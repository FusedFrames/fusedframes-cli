import { mkdirSync, readFileSync, writeFileSync } from "fs";
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
  let data: string;
  try {
    data = readFileSync(getConfigPath(), "utf-8");
  } catch {
    // No config file yet (ENOENT) or unreadable: treat as empty config.
    return {};
  }
  try {
    return JSON.parse(data) as Config;
  } catch {
    // The file exists but is corrupt/unparseable. Don't silently pretend there
    // is no config — warn (to stderr, so JSON stdout stays clean) and continue.
    process.stderr.write(
      `Warning: config file at ${getConfigPath()} is not valid JSON and was ignored. ` +
        `Re-set your key with: echo "ff_..." | fusedframes config set-key\n`
    );
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
}

/** True if the stored config file currently holds an apiKey. */
export function hasStoredApiKey(): boolean {
  return Boolean(readConfig().apiKey);
}

/** Remove the stored apiKey, preserving any other config. Used by `logout`. */
export function clearApiKey(): void {
  const { apiKey: _removed, ...rest } = readConfig();
  void _removed;
  writeConfig(rest);
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
