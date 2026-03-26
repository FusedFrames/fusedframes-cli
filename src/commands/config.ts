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
    .command("set-key [apiKey]")
    .description("Set the API key (reads from stdin if not provided)")
    .action(async (apiKey?: string) => {
      let key = apiKey;

      // If no argument provided, read from stdin
      if (!key) {
        key = await readStdin();
      }

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
