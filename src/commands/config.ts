import { Command } from "commander";
import { readConfig, writeConfig, getConfigInfo } from "../lib/config.js";
import { outputSuccess, outputError } from "../lib/output.js";

// Read a piped/redirected API key from stdin (the recommended, history-safe
// path: `echo "ff_..." | fusedframes config set-key`).
function readPipedStdin(): Promise<string> {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf-8");
    process.stdin.on("data", (chunk) => (data += chunk));
    process.stdin.on("end", () => resolve(data.trim()));
    process.stdin.on("error", reject);
  });
}

// Read the key from an interactive terminal WITHOUT echoing it. The key is
// sensitive: echoing it leaves it visible on screen and in terminal
// scrollback, which is a real exposure for a screen-recording product (screen
// shares, recordings). We put the TTY into raw mode, suppress echo, and accept
// the key on Enter.
function readTtyNoEcho(): Promise<string> {
  return new Promise((resolve, reject) => {
    const stdin = process.stdin;
    process.stderr.write("Enter API key (input hidden): ");
    stdin.setEncoding("utf-8");

    const wasRaw = stdin.isRaw ?? false;
    stdin.setRawMode(true);
    stdin.resume();

    const ENTER_LF = 0x0a;
    const ENTER_CR = 0x0d;
    const EOT = 0x04; // Ctrl-D
    const ETX = 0x03; // Ctrl-C
    const BACKSPACE = 0x08;
    const DELETE = 0x7f;

    let buf = "";
    const cleanup = () => {
      stdin.removeListener("data", onData);
      stdin.setRawMode(wasRaw);
      stdin.pause();
    };
    const onData = (chunk: string) => {
      for (const ch of chunk) {
        const code = ch.charCodeAt(0);
        if (code === ENTER_LF || code === ENTER_CR || code === EOT) {
          cleanup();
          process.stderr.write("\n");
          resolve(buf.trim());
          return;
        }
        if (code === ETX) {
          cleanup();
          process.stderr.write("\n");
          reject(new Error("Aborted"));
          return;
        }
        if (code === BACKSPACE || code === DELETE) {
          buf = buf.slice(0, -1);
          continue;
        }
        // Keep printable input only; drop other control characters.
        if (code >= 0x20) buf += ch;
      }
    };
    stdin.on("data", onData);
  });
}

function readStdin(): Promise<string> {
  // Interactive terminal: prompt with echo disabled. Otherwise read the piped
  // key from stdin.
  return process.stdin.isTTY ? readTtyNoEcho() : readPipedStdin();
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

      const key = await readStdin();

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
