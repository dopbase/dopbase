import { defineQueryOptions } from "@pinia/colada";
import { listTokens } from "~/services/tokens.api";
import { keys } from "./keys";

/** One environment's runner tokens. The plaintext of a new one is never kept. */
export const runnerTokensQuery = defineQueryOptions(
  (environmentId: string) => ({
    key: keys.tokens.of(environmentId),
    query: ({ signal }) => listTokens(environmentId, signal),
  }),
);
