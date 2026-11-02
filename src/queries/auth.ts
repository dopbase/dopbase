import { defineQueryOptions } from "@pinia/colada";
import { fetchAuthMethods } from "~/services/auth.api";
import { keys } from "./keys";
import { RARELY_CHANGES } from "./options";

/** Which sign-in methods the server offers, for signed-in screens. */
export const authMethodsQuery = defineQueryOptions({
  key: keys.authMethods,
  query: ({ signal }) => fetchAuthMethods(signal),
  staleTime: RARELY_CHANGES,
});

/**
 * The same, for screens shown before anyone signs in. A separate entry, so
 * nothing read before sign-in carries over, and dropped as soon as no
 * screen shows it.
 */
export const publicAuthMethodsQuery = defineQueryOptions({
  key: keys.public.authMethods,
  query: ({ signal }) => fetchAuthMethods(signal),
  staleTime: RARELY_CHANGES,
  gcTime: 0,
});
