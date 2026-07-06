import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";
import type { DocumentSummary, DocumentDetail, SourceRecording } from "../lib/types.js";

export function registerDocumentCommands(program: Command): void {
  const documents = program
    .command("documents")
    .description("Query documents");

  documents
    .command("list <libraryId>")
    .description("List documents in a library")
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
          documents: DocumentSummary[];
          total: number;
          page: number;
          pageSize: number;
        }>(`/libraries/${libraryId}/documents`, {
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

  documents
    .command("get <id>")
    .description("Get full document detail with inline edges")
    .action(async (id: string) => {
      const data = await request<DocumentDetail>(`/documents/${id}`);
      outputSuccess(data);
    });

  documents
    .command("source-recordings <id>")
    .description("Get the source recordings behind a document")
    .option("--page <number>", "Page number", "1")
    .option("--page-size <number>", "Results per page", "20")
    .action(
      async (id: string, opts: { page: string; pageSize: string }) => {
        const data = await request<{
          sourceRecordings: SourceRecording[];
          total: number;
          page: number;
          pageSize: number;
        }>(`/documents/${id}/source-recordings`, {
          page: opts.page,
          pageSize: opts.pageSize,
        });
        outputSuccess(data);
      }
    );

}
