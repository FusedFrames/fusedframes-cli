import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";

export function registerTraverseCommand(program: Command): void {
  program
    .command("traverse <patternId>")
    .description("Traverse edges from a pattern")
    .option(
      "--direction <value>",
      "Traversal direction (outgoing, incoming, both)",
      "both"
    )
    .option("--label <value>", "Filter by edge label")
    .option("--depth <number>", "Traversal depth (1-3)", "1")
    .action(
      async (
        patternId: string,
        opts: { direction: string; label?: string; depth: string }
      ) => {
        const data = await request(`/patterns/${patternId}/traverse`, {
          direction: opts.direction,
          label: opts.label,
          depth: opts.depth,
        });
        outputSuccess(data);
      }
    );
}
