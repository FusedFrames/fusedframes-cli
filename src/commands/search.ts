import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";
import type { SearchResult } from "../lib/types.js";

export function registerSearchCommand(program: Command): void {
  program
    .command("search <query>")
    .description("Search documents across all accessible libraries")
    .option("--category <value>", "Filter by category")
    .option("--tag <value>", "Filter by tag")
    .option("--library <value>", "Filter by library ID")
    .option("--page <number>", "Page number", "1")
    .option("--page-size <number>", "Results per page", "20")
    .action(
      async (
        query: string,
        opts: {
          category?: string;
          tag?: string;
          library?: string;
          page: string;
          pageSize: string;
        }
      ) => {
        const data = await request<SearchResult>("/search/documents", {
          q: query,
          category: opts.category,
          tag: opts.tag,
          libraryId: opts.library,
          page: opts.page,
          pageSize: opts.pageSize,
        });
        outputSuccess(data);
      }
    );
}
