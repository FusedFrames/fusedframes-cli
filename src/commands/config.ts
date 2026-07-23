import { Command } from "commander";
import {
  readConfig,
  writeConfig,
  getConfigInfo,
  clearApiKey,
  hasStoredApiKey,
} from "../lib/config.js";
import { outputSuccess, outputError } from "../lib/output.js";

// Control characters read from a raw-mode TTY, by code point.
const ENTER_LF = 10; // \n
const ENTER_CR = 13; // \r
const CTRL_C = 3; // ETX — abort
const CTRL_D = 4; // EOT — end of input
const BACKSPACE = 8; // \b
const DELETE = 127; // DEL

function readKeyFromStdin(): Promise<string> {
  // Piped / redirected input (the documented
  // `echo "ff_..." | fusedframes config set-key` path): read to EOF.
  if (!process.stdin.isTTY) {
    return new Promise((resolve, reject) => {
      let data = "";
      process.stdin.setEncoding("utf-8");
      process.stdin.on("data", (chunk) => (data += chunk));
      process.stdin.on("end", () => resolve(data.trim()));
      process.stdin.on("error", reject);
    });
  }

  // Interactive terminal: read a single line and resolve on Enter (not only on
  // EOF, which looked like a hang), and don't echo the key back to the screen.
  return new Promise((resolve, reject) => {
    const stdin = process.stdin;
    process.stderr.write("Paste your API key and press Enter: ");
    stdin.setEncoding("utf-8");
    stdin.setRawMode?.(true);
    stdin.resume();

    let data = "";
    const cleanup = () => {
      stdin.setRawMode?.(false);
      stdin.pause();
      stdin.removeListener("data", onData);
      stdin.removeListener("error", onError);
    };
    const finish = () => {
      process.stderr.write("\n");
      cleanup();
      resolve(data.trim());
    };
    const onData = (chunk: string) => {
      for (const ch of chunk) {
        const code = ch.charCodeAt(0);
        if (code === ENTER_LF || code === ENTER_CR || code === CTRL_D) {
          finish();
          return;
        }
        if (code === CTRL_C) {
          process.stderr.write("\n");
          cleanup();
          process.exit(130);
        }
        if (code === BACKSPACE || code === DELETE) {
          data = data.slice(0, -1);
          continue;
        }
        data += ch;
      }
    };
    const onError = (err: Error) => {
      cleanup();
      reject(err);
    };
    stdin.on("data", onData);
    stdin.on("error", onError);
  });
}

export function registerConfigCommands(program: Command): void {
  const config = program.command("config").description("Manage CLI configuration");

  config
    .command("set-key")
    .allowExcessArguments(true)
    .description("Set the API key (reads from stdin)")
    .action(async (_options: unknown, command: Command) => {
      // Never accept the key on the command line: argv is recorded in shell
      // history and visible to other processes via ps.
      if (command.args.length > 0) {
        outputError(
          "validation_error",
          'API keys must not be passed as command-line arguments because they are saved in shell history and visible in process listings. Pipe the key via stdin instead: echo "ff_..." | fusedframes config set-key. Alternatively set the FUSEDFRAMES_API_KEY environment variable.'
        );
      }

      const key = await readKeyFromStdin();

      if (!key) {
        outputError("validation_error", "No API key provided");
      }

      const existing = readConfig();
      writeConfig({ ...existing, apiKey: key });
      outputSuccess({ success: true, message: "API key saved" });
    });

  config
    .command("show")
    .description("Show current configuration")
    .action(() => {
      outputSuccess(getConfigInfo());
    });
}

export function registerLogoutCommand(program: Command): void {
  program
    .command("logout")
    .alias("clear-key")
    .description("Remove the stored API key from this machine")
    .action(() => {
      const had = hasStoredApiKey();
      clearApiKey();

      const message = had
        ? "Stored API key removed."
        : "No stored API key to remove.";

      // The env var overrides the stored key, so clearing the file doesn't fully
      // log you out while it's set — say so plainly.
      if (process.env.FUSEDFRAMES_API_KEY) {
        outputSuccess({
          success: true,
          message,
          warning:
            "The FUSEDFRAMES_API_KEY environment variable is still set and takes precedence over the stored key. Unset it in your shell to fully sign out.",
        });
      } else {
        outputSuccess({ success: true, message });
      }
    });
}
