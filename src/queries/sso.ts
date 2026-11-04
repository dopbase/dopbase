import { defineQueryOptions } from "@pinia/colada";
import { fetchSsoIdentity } from "~/services/auth.api";
import { fetchSsoSettings } from "~/services/sso.api";
import { keys } from "./keys";

/**
 * The single sign-on provider and presets, root only. The client secret is
 * never part of it: the server never sends it back.
 */
export const ssoSettingsQuery = defineQueryOptions({
  key: keys.sso.settings,
  query: ({ signal }) => fetchSsoSettings(signal),
});

/** The signed-in account's own single sign-on link. */
export const ssoIdentityQuery = defineQueryOptions({
  key: keys.sso.identity,
  query: ({ signal }) => fetchSsoIdentity(signal),
});
