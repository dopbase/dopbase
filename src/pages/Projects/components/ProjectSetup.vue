<script setup lang="ts">
import { computed, toRef } from "vue";
import type { RouteLocationRaw } from "vue-router";
import {
  CheckIcon,
  ChevronRightIcon,
  PlusIcon,
  SlidersHorizontalIcon,
  UsersIcon,
  XIcon,
} from "~/assets/icons";
import { DbButton, DbCheckbox, DbInput, DbSwitch } from "~/components/ui";
import { useProjectSetupController } from "./ProjectSetup.controller";
import CliCommands from "./CliCommands.vue";

/**
 * ProjectSetup — what a project manager sees in a project with no
 * environments: the setup timeline from the first-run checklist, with
 * creating environments as the current step. It starts from development,
 * staging and production, and Access and Settings sit beside it, since
 * they're the only tabs until an environment exists.
 */
const props = defineProps<{
  projectName: string;
  /** Where Access lives, when you can open it. */
  accessTo?: RouteLocationRaw | null;
  /** Where Settings lives, when you can open it. */
  settingsTo?: RouteLocationRaw | null;
}>();

const emit = defineEmits<{ created: [environmentName: string] }>();

const {
  rows,
  newName,
  newNameError,
  creating,
  created,
  firstName,
  submitLabel,
  canCreate,
  addRow,
  removeRow,
  editNewName,
  create,
  openCreated,
} = useProjectSetupController(toRef(props, "projectName"), (name) =>
  emit("created", name),
);

const createCommand = computed(() => [
  `dopbase env create ${props.projectName}/${firstName.value}`,
]);
const runCommands = computed(() => [
  "dopbase login",
  `dopbase run ${props.projectName}/${firstName.value} -- npm run dev`,
]);
/** Some made it and one failed: offer to go on with what exists. */
const stopped = computed(
  () =>
    !creating.value &&
    created.value.length > 0 &&
    rows.value.some((row) => row.status === "failed"),
);
</script>

<template>
  <div
    class="flex flex-col gap-10 xl:flex-row xl:items-start xl:gap-12"
    data-testid="project-setup">
    <section
      class="flex min-w-0 max-w-[720px] flex-1 flex-col gap-7 py-4"
      aria-labelledby="project-setup-title">
      <header class="flex flex-col gap-1.5">
        <h2
          id="project-setup-title"
          class="text-lg font-semibold leading-[26px] text-ink-strong">
          Set up {{ projectName }}
        </h2>
        <p class="text-[15px] leading-6 text-ink-muted">
          Start with where its secrets will live. This list goes away once an
          environment has its first secret.
        </p>
        <div class="mt-1.5 flex items-center gap-3">
          <div
            class="h-1 max-w-[600px] flex-1 overflow-hidden rounded-full bg-line"
            role="progressbar"
            aria-label="Setup progress"
            aria-valuemin="0"
            aria-valuemax="4"
            aria-valuenow="1">
            <div class="h-full w-1/4 rounded-full bg-ok" />
          </div>
          <span class="shrink-0 text-sm leading-[22px] text-ink-muted">
            1 of 4 done
          </span>
        </div>
      </header>

      <ol class="flex flex-col">
        <li class="relative flex gap-4 pb-6">
          <span
            class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-ok/40"
            aria-hidden="true" />
          <span
            class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
            aria-hidden="true">
            <CheckIcon class="h-4 w-4" />
          </span>
          <div class="min-w-0">
            <p class="text-[15px] font-semibold leading-5 text-ink-strong">
              Project created
              <span class="sr-only">(done)</span>
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              <span class="font-mono">{{ projectName }}</span> is ready, but it
              has no environments yet, so there’s nowhere to put secrets.
            </p>
          </div>
        </li>

        <li class="relative flex gap-4 pb-6" aria-current="step">
          <span
            class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-line"
            aria-hidden="true" />
          <span
            class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control border-[1.5px] border-ink-strong text-sm font-semibold text-ink-strong"
            aria-hidden="true">
            2
          </span>
          <div class="flex min-w-0 flex-1 flex-col gap-2.5">
            <div>
              <p class="text-[15px] font-semibold leading-5 text-ink-strong">
                Create environments
              </p>
              <p class="mt-1 text-sm leading-[22px] text-ink-muted">
                Each environment keeps its own values. Most teams start with
                these three. Change them now or later in Settings.
              </p>
            </div>

            <ul
              class="overflow-hidden rounded-card border border-line bg-panel"
              aria-label="Environments to create"
              data-testid="setup-environments">
              <li
                v-for="row in rows"
                :key="row.name"
                class="flex items-center gap-3 border-b border-line-soft px-4 py-3.5"
                :data-testid="`setup-row-${row.name}`">
                <DbCheckbox
                  :model-value="row.selected || row.status === 'created'"
                  :aria-label="`Create ${row.name}`"
                  :disabled="creating || row.status === 'created'"
                  @update:model-value="row.selected = $event" />
                <div class="min-w-0 flex-1">
                  <p class="flex flex-wrap items-center gap-x-2">
                    <span
                      class="truncate font-mono text-sm font-medium leading-[22px] text-ink-strong">
                      {{ row.name }}
                    </span>
                    <span
                      v-if="row.status === 'created'"
                      class="inline-flex items-center gap-1 text-[13px] leading-5 text-ok">
                      <CheckIcon class="h-3.5 w-3.5" />
                      Created
                    </span>
                    <span
                      v-else-if="row.status === 'creating'"
                      class="text-[13px] leading-5 text-ink-muted">
                      Creating…
                    </span>
                  </p>
                  <p
                    v-if="row.description"
                    class="text-[13px] leading-5 text-ink-muted">
                    {{ row.description }}
                  </p>
                  <p
                    v-if="row.error"
                    class="text-[13px] leading-5"
                    :class="row.status === 'failed' ? 'text-crit' : 'text-warn'"
                    role="alert">
                    {{ row.error }}
                  </p>
                </div>
                <DbSwitch
                  v-model="row.protected"
                  label="Protected"
                  :disabled="creating || row.status === 'created'" />
                <button
                  v-if="row.custom && row.status !== 'created'"
                  type="button"
                  class="flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong disabled:cursor-default disabled:opacity-50"
                  :aria-label="`Take ${row.name} off the list`"
                  :disabled="creating"
                  @click="removeRow(row.name)">
                  <XIcon class="h-4 w-4" />
                </button>
              </li>
              <li class="px-4 py-3">
                <form
                  class="flex items-start gap-2"
                  data-testid="setup-add"
                  @submit.prevent="addRow">
                  <div class="min-w-0 flex-1">
                    <DbInput
                      :model-value="newName"
                      placeholder="Add another, like qa or preview"
                      mono
                      :error="newNameError"
                      :disabled="creating"
                      autocomplete="off"
                      @update:model-value="editNewName" />
                  </div>
                  <DbButton type="submit" :disabled="creating">
                    <PlusIcon class="h-4 w-4" />
                    Add
                  </DbButton>
                </form>
              </li>
            </ul>

            <div class="flex flex-wrap items-center gap-3 pt-1">
              <DbButton
                variant="primary"
                :disabled="!canCreate"
                :loading="creating"
                data-testid="setup-create"
                @click="create">
                {{ submitLabel }}
              </DbButton>
              <DbButton v-if="stopped" @click="openCreated">
                Open {{ created[0]?.name }}
              </DbButton>
              <p v-else class="text-[13px] leading-5 text-ink-muted">
                You’ll land in
                <span class="font-mono">{{ firstName }}</span
                >, ready for its first secret.
              </p>
            </div>

            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              Or from your terminal
            </p>
            <CliCommands :commands="createCommand" />
          </div>
        </li>

        <li class="relative flex gap-4 pb-6">
          <span
            class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-line"
            aria-hidden="true" />
          <span
            class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control border border-line-strong text-sm font-semibold text-ink-muted"
            aria-hidden="true">
            3
          </span>
          <div class="min-w-0">
            <p class="text-[15px] font-semibold leading-5 text-ink">
              Add secrets
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-faint">
              Import the <span class="font-mono">.env</span> you already have,
              or type them in. Ready once an environment exists.
            </p>
          </div>
        </li>

        <li class="flex gap-4">
          <span
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-control border border-line-strong text-sm font-semibold text-ink-muted"
            aria-hidden="true">
            4
          </span>
          <div class="flex min-w-0 flex-1 flex-col gap-2.5">
            <div>
              <p class="text-[15px] font-semibold leading-5 text-ink">
                Run your app with Dopbase
              </p>
              <p class="mt-1 text-sm leading-[22px] text-ink-faint">
                Run it through the CLI and the secrets arrive as environment
                variables.
              </p>
            </div>
            <CliCommands :commands="runCommands" dim />
          </div>
        </li>
      </ol>
    </section>

    <aside
      v-if="accessTo || settingsTo"
      class="flex flex-col gap-3 xl:w-[300px] xl:shrink-0 xl:pt-4"
      aria-label="While you’re here">
      <p class="text-[13px] font-medium leading-5 text-ink-muted">
        While you’re here
      </p>
      <RouterLink
        v-if="accessTo"
        :to="accessTo"
        class="group flex flex-col gap-1.5 rounded-card border border-line bg-panel p-4 transition-colors hover:border-line-strong"
        data-testid="setup-access">
        <span class="flex items-center gap-2">
          <UsersIcon class="h-4 w-4 text-ink-muted" />
          <span class="text-sm font-semibold leading-[22px] text-ink-strong">
            Give your team access
          </span>
        </span>
        <span class="text-[13px] leading-5 text-ink-muted">
          Grants on the project reach every environment you add, except
          protected ones.
        </span>
        <span
          class="flex items-center gap-1 pt-1 text-[13px] font-medium leading-5 text-accent-strong">
          Open Access
          <ChevronRightIcon class="h-3.5 w-3.5" />
        </span>
      </RouterLink>
      <RouterLink
        v-if="settingsTo"
        :to="settingsTo"
        class="group flex flex-col gap-1.5 rounded-card border border-line bg-panel p-4 transition-colors hover:border-line-strong"
        data-testid="setup-settings">
        <span class="flex items-center gap-2">
          <SlidersHorizontalIcon class="h-4 w-4 text-ink-muted" />
          <span class="text-sm font-semibold leading-[22px] text-ink-strong">
            Project settings
          </span>
        </span>
        <span class="text-[13px] leading-5 text-ink-muted">
          Rename the project or delete it. Nothing is stored in it yet.
        </span>
        <span
          class="flex items-center gap-1 pt-1 text-[13px] font-medium leading-5 text-accent-strong">
          Open Settings
          <ChevronRightIcon class="h-3.5 w-3.5" />
        </span>
      </RouterLink>
    </aside>
  </div>
</template>
