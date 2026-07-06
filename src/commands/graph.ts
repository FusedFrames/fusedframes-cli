import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";

export function registerGraphCommand(program: Command): void {
  program
    .command("graph <libraryId>")
    .description("Get the full document graph for a library")
    .action(async (libraryId: string) => {
      const data = await request(`/libraries/${libraryId}/graph`);
      outputSuccess(data);
    });
}
