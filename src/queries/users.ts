import { defineQueryOptions } from "@pinia/colada";
import {
  fetchUser,
  fetchUserIdentities,
  fetchUsers,
} from "~/services/users.api";
import { fetchRoles } from "~/services/roles.api";
import { keys } from "./keys";
import { RARELY_CHANGES } from "./options";

/** Every account, people and AI agents alike. */
export const usersQuery = defineQueryOptions({
  key: keys.users.list,
  query: ({ signal }) => fetchUsers(signal),
});

/** One account. */
export const userQuery = defineQueryOptions((userId: string) => ({
  key: keys.users.one(userId),
  query: ({ signal }) => fetchUser(userId, signal),
}));

/** Every account's id and email, for filters. */
export const userIdentitiesQuery = defineQueryOptions({
  key: keys.users.identities,
  query: ({ signal }) => fetchUserIdentities(signal),
});

/** Built-in and custom roles. They rarely change. */
export const rolesQuery = defineQueryOptions({
  key: keys.roles,
  query: ({ signal }) => fetchRoles(signal),
  staleTime: RARELY_CHANGES,
});
