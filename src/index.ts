#!/usr/bin/env node

import { Command } from "commander";
import { registerConfigCommands } from "./commands/config.js";
import { registerLibraryCommands } from "./commands/libraries.js";
import { registerPatternCommands } from "./commands/patterns.js";
import { registerGraphCommand } from "./commands/graph.js";
import { registerTraverseCommand } from "./commands/traverse.js";
import { registerSearchCommand } from "./commands/search.js";
import { outputError } from "./lib/output.js";
import { FusedFramesError } from "./lib/client.js";

const program = new Command();

program
  .name("fusedframes")
  .description("Query FusedFrames behavioural patterns")
  .version("1.0.0");

// Register all command groups
registerConfigCommands(program);
registerLibraryCommands(program);
registerPatternCommands(program);
registerGraphCommand(program);
registerTraverseCommand(program);
registerSearchCommand(program);

// Override commander to throw instead of exit, but keep help/version output
program.exitOverride();

// Global error handler
async function main() {
  try {
    await program.parseAsync(process.argv);
  } catch (error) {
    if (error instanceof FusedFramesError) {
      outputError(error.code, error.message);
    } else if (error instanceof Error) {
      // Commander exits with code 'commander.helpDisplayed' or 'commander.version'
      const commanderError = error as Error & { code?: string };
      if (
        commanderError.code === "commander.helpDisplayed" ||
        commanderError.code === "commander.version"
      ) {
        // These are expected — commander already wrote output
        process.exit(0);
      }
      if (commanderError.code === "commander.missingArgument" ||
          commanderError.code === "commander.unknownCommand" ||
          commanderError.code === "commander.unknownOption" ||
          commanderError.code === "commander.missingMandatoryOptionValue") {
        outputError("validation_error", error.message);
      }
      outputError("error", error.message);
    } else {
      outputError("error", "An unexpected error occurred");
    }
  }
}

main();
