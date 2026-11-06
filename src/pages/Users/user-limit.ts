import { computed, type ComputedRef } from "vue";
import { useQuery } from "@pinia/colada";
import { onlyWhen } from "~/queries/helpers";
import { usersQuery } from "~/queries/users";
import { useAuthStore } from "~/stores/auth.store";
import { userLimit, type UserLimit } from "./people";

/**
 * How full the server's people limit is, or null when there is none.
 * Counts the users list, which the Users page already loads, and reads it
 * only when the session says a limit is set.
 */
export function useUserLimit(): ComputedRef<UserLimit | null> {
  const auth = useAuthStore();
  const accounts = useQuery(() =>
    onlyWhen(!!auth.session?.maxUsers && auth.can("users.view"), usersQuery),
  );
  return computed(() =>
    accounts.data.value
      ? userLimit(accounts.data.value, auth.session?.maxUsers)
      : null,
  );
}
