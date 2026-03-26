import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";
import type { PatternSummary, PatternDetail, EvidenceAction } from "../lib/types.js";

export function registerPatternCommands(program: Command): void {
  const patterns = program
    .command("patterns")
    .description("Query patterns");

  patterns
    .command("list <libraryId>")
    .description("List patterns in a library")
    .option("--category <value>", "Filter by category")
    .option("--tag <value>", "Filter by tag")
    .option("--app <value>", "Filter by application")
    .option("--search <value>", "Search term")
    .option("--page <number>", "Page number", "1")
    .option("--page-size <number>", "Results per page", "20")
    .action(
      async (
        libraryId: string,
        opts: {
          category?: string;
          tag?: string;
          app?: string;
          search?: string;
          page: string;
          pageSize: string;
        }
      ) => {
        const data = await request<{
          patterns: PatternSummary[];
          total: number;
          page: number;
          pageSize: number;
        }>(`/libraries/${libraryId}/patterns`, {
          category: opts.category,
          tag: opts.tag,
          application: opts.app,
          search: opts.search,
          page: opts.page,
          pageSize: opts.pageSize,
        });
        outputSuccess(data);
      }
    );

  patterns
    .command("get <id>")
    .description("Get full pattern detail with inline edges")
    .action(async (id: string) => {
      const data = await request<PatternDetail>(`/patterns/${id}`);
      outputSuccess(data);
    });

  patterns
    .command("evidence <id>")
    .description("Get evidence actions for a pattern")
    .option("--page <number>", "Page number", "1")
    .option("--page-size <number>", "Results per page", "20")
    .action(
      async (id: string, opts: { page: string; pageSize: string }) => {
        const data = await request<{
          evidence: EvidenceAction[];
          total: number;
          page: number;
          pageSize: number;
        }>(`/patterns/${id}/evidence`, {
          page: opts.page,
          pageSize: opts.pageSize,
        });
        outputSuccess(data);
      }
    );

}
