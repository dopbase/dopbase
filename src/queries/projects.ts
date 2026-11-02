import { defineQueryOptions, useQueryCache } from "@pinia/colada";
import { listProjects } from "~/services/projects.api";
import { listEnvironments } from "~/services/environments.api";
import type { SecretMetadata } from "~/services/secrets.api";
import type { RunnerToken } from "~/services/tokens.api";
import { keys } from "./keys";
import { readQuery } from "./helpers";
import { secretsQuery } from "./secrets";
import { runnerTokensQuery } from "./tokens";

/** Every project the viewer reaches. */
export const projectsQuery = defineQueryOptions({
  key: keys.projects,
  query: ({ signal }) => listProjects(signal),
});

/**
 * The environments of one project, by name or id, or of every project the
 * viewer reaches when `project` is `null`.
 */
export const environmentsQuery = defineQueryOptions(
  (project: string | null) => ({
    key: keys.environments.list(project),
    query: ({ signal }) => listEnvironments(project ?? undefined, signal),
  }),
);

/** One environment's secrets and runner tokens; `null` when unreadable. */
export interface EnvironmentContents {
  secrets: SecretMetadata[] | null;
  tokens: RunnerToken[] | null;
}

/**
 * The secrets and runner tokens of several environments, read through the
 * same entries the Secrets and Tokens tabs use, so opening a tab afterwards
 * needs no new request. A list that fails to load is `null`.
 */
export const environmentContentsQuery = defineQueryOptions(
  ({
    cache,
    environmentIds,
  }: {
    cache: ReturnType<typeof useQueryCache>;
    environmentIds: readonly string[];
  }) => ({
    key: keys.environmentStats.of(environmentIds),
    query: async ({ signal }): Promise<Record<string, EnvironmentContents>> => {
      const rows = await Promise.all(
        environmentIds.map(async (id) => {
          const [secrets, tokens] = await Promise.allSettled([
            readQuery(cache, secretsQuery(id), { signal }),
            readQuery(cache, runnerTokensQuery(id), { signal }),
          ]);
          const contents: EnvironmentContents = {
            secrets: secrets.status === "fulfilled" ? secrets.value : null,
            tokens: tokens.status === "fulfilled" ? tokens.value : null,
          };
          return [id, contents] as const;
        }),
      );
      return Object.fromEntries(rows);
    },
  }),
);
