import { defineInfiniteQueryOptions, defineQueryOptions } from "@pinia/colada";
import {
  listSecrets,
  listSecretVersions,
  type SecretVersionPage,
} from "~/services/secrets.api";
import { keys } from "./keys";

/**
 * One environment's secret keys and metadata. Values are never part of it:
 * reveals, exports and the editor read them with one-off calls.
 */
export const secretsQuery = defineQueryOptions((environmentId: string) => ({
  key: keys.secrets.of(environmentId),
  query: ({ signal }) => listSecrets(environmentId, signal),
}));

/**
 * One secret's versions, newest first, a page at a time. Metadata only; an
 * earlier value is revealed with its own call. Like the audit feed, it
 * doesn't refresh on window focus, which would refetch every page loaded.
 */
export const secretVersionsQuery = defineInfiniteQueryOptions(
  ({
    environmentId,
    key,
    limit,
  }: {
    environmentId: string;
    key: string;
    limit: number;
  }) => ({
    key: keys.secretVersions.of(environmentId, key),
    initialPageParam: null as number | null,
    // A parked history has no pages to read a cursor from.
    getNextPageParam: (page: SecretVersionPage | undefined) =>
      page?.nextBefore ?? null,
    query: ({ signal, pageParam }) =>
      listSecretVersions(environmentId, key, {
        limit,
        signal,
        ...(pageParam === null ? {} : { before: pageParam }),
      }),
    refetchOnWindowFocus: false,
  }),
);
