import { computed, ref, watch, type Ref } from "vue";
import { updateUser } from "~/services/users.api";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";

/**
 * Active people other than the agent's current owner who can see it, and so
 * could own it: root, or a role with `users.view`.
 */
export function transferCandidates(
  people: User[],
  roles: Role[],
  agent: User | null,
): User[] {
  const seesUsers = new Set(
    roles
      .filter((role) => role.capabilities.includes("users.view"))
      .map((role) => role.id),
  );
  return people.filter(
    (person) =>
      person.role !== "ai_agent" &&
      person.status !== "disabled" &&
      person.id !== agent?.ownerId &&
      (person.role === "root" || seesUsers.has(person.roleId ?? "")),
  );
}

/**
 * Transfer owner: hand an AI agent to another person. The agent then never
 * reaches more than its new owner does.
 */
export function useTransferOwnerDialog(
  open: Ref<boolean>,
  agent: Ref<User | null>,
  people: Ref<User[]>,
  roles: Ref<Role[]>,
) {
  const { runWithReauth } = useReauthentication();
  const ownerId = ref("");
  const saving = ref(false);
  const error = ref<string | null>(null);

  const options = computed(() =>
    transferCandidates(people.value, roles.value, agent.value).map(
      (person) => ({ value: person.id, label: person.email }),
    ),
  );

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      error.value = null;
      ownerId.value = options.value[0]?.value ?? "";
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  async function submit(): Promise<boolean> {
    const target = agent.value;
    if (!target || !ownerId.value) {
      error.value = "Choose the new owner.";
      return false;
    }
    saving.value = true;
    error.value = null;
    try {
      await runWithReauth(async () => {
        await updateUser(target.id, { ownerId: ownerId.value });
      });
      return true;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError")
        return false;
      error.value = errorMessage(cause, "Could not transfer the agent.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  return { ownerId, options, saving, error, submit };
}
