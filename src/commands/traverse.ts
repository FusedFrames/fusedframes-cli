import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";

export function registerTraverseCommand(program: Command): void {
  program
    .command("traverse <documentId>")
    .description("Traverse edges from a document")
    .option(
      "--direction <value>",
      "Traversal direction (outgoing, incoming, both)",
      "both"
    )
    .option("--label <value>", "Filter by edge label")
    .option("--depth <number>", "Traversal depth (1-3)", "1")
    .action(
      async (
        documentId: string,
        opts: { direction: string; label?: string; depth: string }
      ) => {
        const data = await request(`/documents/${documentId}/traverse`, {
          direction: opts.direction,
          label: opts.label,
          depth: opts.depth,
        });
        outputSuccess(data);
      }
    );
}
