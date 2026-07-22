import { Command } from "commander";
import { readConfig, writeConfig, getConfigInfo } from "../lib/config.js";
import { outputSuccess, outputError } from "../lib/output.js";

function readStdin(): Promise<string> {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf-8");
    process.stdin.on("data", (chunk) => (data += chunk));
    process.stdin.on("end", () => resolve(data.trim()));
    process.stdin.on("error", reject);

    // If stdin is a TTY (interactive terminal), prompt the user
    if (process.stdin.isTTY) {
      process.stderr.write("Enter API key: ");
    }
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
