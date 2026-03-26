import { Command } from "commander";
import { request } from "../lib/client.js";
import { outputSuccess } from "../lib/output.js";
import type { LibrarySummary, LibraryDetail, CategoryCount, TagCount, ApplicationCount } from "../lib/types.js";

export function registerLibraryCommands(program: Command): void {
  const libraries = program
    .command("libraries")
    .description("Browse pattern libraries");

  libraries
    .command("list")
    .description("List all accessible pattern libraries")
    .action(async () => {
      const data = await request<{ libraries: LibrarySummary[] }>("/libraries");
      outputSuccess(data);
    });

  libraries
    .command("get <id>")
    .description("Get pattern library detail")
    .action(async (id: string) => {
      const data = await request<LibraryDetail>(`/libraries/${id}`);
      outputSuccess(data);
    });

  libraries
    .command("categories <id>")
    .description("List categories with pattern counts")
    .action(async (id: string) => {
      const data = await request<{ categories: CategoryCount[] }>(
        `/libraries/${id}/categories`
      );
      outputSuccess(data);
    });

  libraries
    .command("tags <id>")
    .description("List tags with pattern counts")
    .action(async (id: string) => {
      const data = await request<{ tags: TagCount[] }>(
        `/libraries/${id}/tags`
      );
      outputSuccess(data);
    });

  libraries
    .command("applications <id>")
    .description("List applications with pattern counts")
    .action(async (id: string) => {
      const data = await request<{ applications: ApplicationCount[] }>(
        `/libraries/${id}/applications`
      );
      outputSuccess(data);
    });
}
