import { computed, ref } from "vue";
import type { Ref } from "vue";
import * as environmentsApi from "~/services/environments.api";
import { errorMessage } from "~/services/api-errors";
import type { Environment } from "~/services";
import { useInvalidate } from "~/queries/invalidate";

/** One environment on the setup list, ticked or not. */
export interface SetupRow {
  name: string;
  /** Empty for the ones added with Add another. */
  description: string;
  protected: boolean;
  selected: boolean;
  /** Added with Add another, so it can be taken off the list again. */
  custom: boolean;
  status: "idle" | "creating" | "created" | "failed";
  /** Why it failed, or what went wrong after it was created. */
  error: string | null;
  /** Set once created. */
  environmentId: string | null;
}

/** The server's rule for environment names. */
const SLUG = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;
const MAX_NAME = 63;

function presets(): SetupRow[] {
  const row = (
    name: string,
    description: string,
    isProtected = false,
  ): SetupRow => ({
    name,
    description,
    protected: isProtected,
    selected: true,
    custom: false,
    status: "idle",
    error: null,
    environmentId: null,
  });
  return [
    row("development", "Laptops and feature branches."),
    row("staging", "A last check before release, set up like production."),
    row(
      "production",
      "Live traffic. While protected, only people granted on it and project managers can open it.",
      true,
    ),
  ];
}

/**
 * Project setup controller: the environments a project manager creates in
 * a project that has none. It starts from development, staging and
 * production (production protected), and Add another takes any name the
 * server accepts.
 *
 * Create makes the ticked ones in list order, one request each, and a
 * protected one is protected by that same request, so it is never open to
 * project grants. A failure keeps what worked, marks the failed row and
 * stops there, so a retry only sends what's left. When everything is in,
 * `onDone` opens the first one created.
 */
export function useProjectSetupController(
  projectName: Ref<string>,
  onDone: (environmentName: string) => void | Promise<void>,
) {
  const invalidate = useInvalidate();
  const rows = ref<SetupRow[]>(presets());
  const newName = ref("");
  const newNameError = ref<string | null>(null);
  const creating = ref(false);

  /** Ticked and not created yet: what Create sends. */
  const pending = computed(() =>
    rows.value.filter((row) => row.selected && row.status !== "created"),
  );
  const created = computed(() =>
    rows.value.filter((row) => row.status === "created"),
  );
  /** Something failed: stay here so the error can be read and retried. */
  const stopped = computed(
    () =>
      !creating.value &&
      created.value.length > 0 &&
      rows.value.some((row) => row.status === "failed"),
  );
  /** The environment the run commands point at. */
  const firstName = computed(
    () =>
      rows.value.find((row) => row.selected)?.name ??
      rows.value[0]?.name ??
      "development",
  );

  const submitLabel = computed(() => {
    const count = pending.value.length;
    if (count === 0) return "Create environments";
    return count === 1
      ? "Create 1 environment"
      : `Create ${count} environments`;
  });
  const canCreate = computed(() => !creating.value && pending.value.length > 0);

  function nameProblem(name: string): string | null {
    if (!name) return "Type a name first.";
    if (name.length > MAX_NAME) return `Keep it to ${MAX_NAME} characters.`;
    if (!SLUG.test(name))
      return "Use lowercase letters, numbers and dashes, starting with a letter.";
    if (rows.value.some((row) => row.name === name))
      return `${name} is already on the list.`;
    return null;
  }

  function addRow(): boolean {
    const name = newName.value.trim();
    const problem = nameProblem(name);
    newNameError.value = problem;
    if (problem) return false;
    rows.value.push({
      name,
      description: "",
      protected: false,
      selected: true,
      custom: true,
      status: "idle",
      error: null,
      environmentId: null,
    });
    newName.value = "";
    return true;
  }

  function removeRow(name: string): void {
    rows.value = rows.value.filter(
      (row) => !(row.custom && row.name === name && row.status !== "created"),
    );
  }

  /** Clears a stale error once the name changes. */
  function editNewName(value: string): void {
    newName.value = value;
    newNameError.value = null;
  }

  async function create(): Promise<void> {
    if (!canCreate.value) return;
    creating.value = true;
    let failed = false;
    for (const row of pending.value) {
      row.status = "creating";
      row.error = null;
      let environment: Environment;
      try {
        environment = await environmentsApi.createEnvironment(
          projectName.value,
          row.name,
          row.protected,
        );
      } catch (cause) {
        row.status = "failed";
        row.error = errorMessage(cause, `Could not create ${row.name}.`);
        failed = true;
        break;
      }
      row.status = "created";
      row.environmentId = environment.id;
    }
    creating.value = false;
    if (!failed) await openCreated();
    // Those made before the failure exist now; every list should show them.
    else if (created.value.length > 0) void invalidate.projectTreeChanged();
  }

  /** Opens the first environment created, skipping any still to retry. */
  async function openCreated(): Promise<void> {
    const first = created.value[0];
    if (first) await onDone(first.name);
  }

  return {
    rows,
    newName,
    newNameError,
    creating,
    pending,
    created,
    stopped,
    firstName,
    submitLabel,
    canCreate,
    addRow,
    removeRow,
    editNewName,
    create,
    openCreated,
  };
}

export type ProjectSetupController = ReturnType<
  typeof useProjectSetupController
>;
