#!/usr/bin/env node

import { Command } from "commander";
import { registerConfigCommands, registerLogoutCommand } from "./commands/config.js";
import { registerLibraryCommands } from "./commands/libraries.js";
import { registerDocumentCommands } from "./commands/documents.js";
import { registerGraphCommand } from "./commands/graph.js";
import { registerTraverseCommand } from "./commands/traverse.js";
import { registerSearchCommand } from "./commands/search.js";
import { outputError } from "./lib/output.js";
import { FusedFramesError } from "./lib/client.js";
import { VERSION } from "./lib/version.js";

const program = new Command();

program
  .name("fusedframes")
  .description("Query documents FusedFrames writes from recorded work")
  .version(VERSION);

// Override commander to throw instead of exit, but keep help/version output.
// Must run BEFORE the commands are registered — subcommands copy this setting
// at creation time, so a later call would leave their errors exiting directly.
program.exitOverride();

// Register all command groups
registerConfigCommands(program);
registerLogoutCommand(program);
registerLibraryCommands(program);
registerDocumentCommands(program);
registerGraphCommand(program);
registerTraverseCommand(program);
registerSearchCommand(program);

// Global error handler
async function main() {
  try {
    await program.parseAsync(process.argv);
  } catch (error) {
    if (error instanceof FusedFramesError) {
      outputError(error.code, error.message);
    } else if (error instanceof Error) {
      // Commander signals non-API outcomes via error codes.
      const commanderError = error as Error & { code?: string };
      if (
        commanderError.code === "commander.helpDisplayed" ||
        commanderError.code === "commander.version"
      ) {
        // These are expected — commander already wrote output
        process.exit(0);
      }
      if (commanderError.code === "commander.help") {
        // Bare `fusedframes` (or a bare subcommand): commander already printed
        // the help text — exit without emitting a JSON error on top of it.
        process.exit(1);
      }
      if (commanderError.code === "commander.missingArgument" ||
          commanderError.code === "commander.unknownCommand" ||
          commanderError.code === "commander.unknownOption" ||
          commanderError.code === "commander.optionMissingArgument") {
        outputError("validation_error", error.message);
      }
      outputError("error", error.message);
    } else {
      outputError("error", "An unexpected error occurred");
    }
  }
}

main();
