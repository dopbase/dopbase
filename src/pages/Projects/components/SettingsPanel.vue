<script setup lang="ts">
import { computed, toRef } from "vue";
import { useSettingsPanelController } from "./SettingsPanel.controller";
import NameDialog from "./NameDialog.vue";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbInput,
  DbMenu,
  DbMenuItem,
  DbSwitch,
} from "~/components/ui";
import { LockIcon, PencilIcon, PlusIcon, TrashIcon } from "~/assets/icons";
import type { Environment, Project } from "~/services";

/**
 * SettingsPanel — the project's Settings tab, for project managers: its
 * name, its environments with protection and counts, and a danger zone.
 */
const props = defineProps<{
  project: Project | null;
  environments: Environment[];
}>();

const emit = defineEmits<{ changed: [] }>();

const controller = useSettingsPanelController(
  toRef(props, "project"),
  toRef(props, "environments"),
  () => emit("changed"),
);
const {
  name,
  nameChanged,
  savingName,
  nameError,
  actionError,
  stats,
  dialog,
  deleting,
  deleteCounts,
  deleteLoading,
  deleteError,
  projectDeleteSummary,
} = controller;

const dialogCopy = computed(() =>
  dialog.value?.kind === "rename-environment"
    ? {
        title: "Rename environment",
        initialName: dialog.value.environment.name,
        submitLabel: "Rename",
        hint: "Links and CLI paths that use the old name stop working.",
      }
    : {
        title: `New environment in ${props.project?.name ?? "this project"}`,
        initialName: "",
        submitLabel: "Create",
        hint: "Example: development, staging, production",
      },
);

const deleteName = computed(() =>
  deleting.value?.kind === "environment"
    ? deleting.value.environment.name
    : (props.project?.name ?? ""),
);

const card = "rounded-card border border-line bg-panel p-4 sm:p-5";
</script>

<template>
  <div class="flex flex-col gap-5">
    <DbAlert v-if="actionError">{{ actionError }}</DbAlert>

    <!-- General -->
    <section :class="card" aria-labelledby="settings-general">
      <form class="flex flex-col gap-4" @submit.prevent="controller.saveName">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2
              id="settings-general"
              class="text-[15px] font-semibold text-ink-strong">
              General
            </h2>
            <p class="mt-0.5 text-sm text-ink-muted">
              The name is used in CLI paths like
              <code class="font-mono">{{ project?.name }}/dev</code>, so
              renaming breaks scripts that use the old one.
            </p>
          </div>
          <DbButton
            variant="primary"
            type="submit"
            :disabled="!nameChanged"
            :loading="savingName">
            Save
          </DbButton>
        </div>
        <DbInput
          v-model="name"
          class="max-w-md"
          label="Name"
          name="projectName"
          mono
          :error="nameError" />
      </form>
    </section>

    <!-- Environments -->
    <section :class="card" aria-labelledby="settings-environments">
      <div class="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2
            id="settings-environments"
            class="text-[15px] font-semibold text-ink-strong">
            Environments
          </h2>
          <p class="mt-0.5 text-sm text-ink-muted">
            Protected environments ignore project-wide grants below Project
            manager.
          </p>
        </div>
        <DbButton
          data-testid="settings-new-environment"
          @click="dialog = { kind: 'new-environment' }">
          <PlusIcon class="h-4 w-4" />
          New environment
        </DbButton>
      </div>
      <div class="overflow-x-auto rounded-control border border-line">
        <table
          class="min-w-full text-left text-sm"
          data-testid="settings-environments">
          <thead>
            <tr class="border-b border-line-soft">
              <th class="h-10 px-4 font-medium text-ink-muted">Environment</th>
              <th class="h-10 px-4 font-medium text-ink-muted">Protected</th>
              <th
                class="hidden h-10 px-4 font-medium text-ink-muted sm:table-cell">
                Secrets
              </th>
              <th
                class="hidden h-10 px-4 font-medium text-ink-muted sm:table-cell">
                Runner tokens
              </th>
              <th class="h-10 w-12 px-2">
                <span class="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="environments.length === 0">
              <td colspan="5" class="px-4 py-6 text-center text-ink-muted">
                No environments yet.
              </td>
            </tr>
            <tr
              v-for="environment in environments"
              :key="environment.id"
              class="border-b border-line-soft last:border-b-0"
              data-testid="settings-environment">
              <td class="h-11 px-4">
                <span
                  class="inline-flex items-center gap-1.5 font-mono text-sm text-ink-strong">
                  {{ environment.name }}
                  <LockIcon
                    v-if="environment.protected"
                    class="h-3.5 w-3.5 text-warn"
                    aria-label="protected" />
                </span>
              </td>
              <td class="h-11 px-4">
                <DbSwitch
                  :model-value="environment.protected ?? false"
                  :label="environment.protected ? 'On' : 'Off'"
                  :disabled="!controller.canManage(environment)"
                  @update:model-value="
                    (value: boolean) =>
                      controller.setProtected(environment, value)
                  " />
              </td>
              <td class="hidden h-11 px-4 text-ink sm:table-cell">
                {{ stats[environment.id]?.secrets ?? "—" }}
              </td>
              <td class="hidden h-11 px-4 text-ink sm:table-cell">
                {{ stats[environment.id]?.tokens ?? "—" }}
              </td>
              <td class="h-11 px-2 text-right">
                <DbMenu
                  v-if="controller.canManage(environment)"
                  :label="`Actions for ${environment.name}`">
                  <DbMenuItem
                    @select="
                      dialog = { kind: 'rename-environment', environment }
                    ">
                    <template #icon><PencilIcon class="h-4 w-4" /></template>
                    Rename
                  </DbMenuItem>
                  <DbMenuItem
                    @select="
                      controller.setProtected(
                        environment,
                        !environment.protected,
                      )
                    ">
                    <template #icon><LockIcon class="h-4 w-4" /></template>
                    {{ environment.protected ? "Unprotect" : "Protect" }}
                  </DbMenuItem>
                  <div class="my-0.5 h-px bg-line-soft" role="separator" />
                  <DbMenuItem
                    tone="danger"
                    @select="
                      controller.startDelete({
                        kind: 'environment',
                        environment,
                      })
                    ">
                    <template #icon><TrashIcon class="h-4 w-4" /></template>
                    Delete environment
                  </DbMenuItem>
                </DbMenu>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <!-- Danger zone -->
    <section
      class="rounded-card border border-crit/60 p-4 sm:p-5"
      aria-labelledby="settings-danger">
      <h2 id="settings-danger" class="mb-3 text-[15px] font-semibold text-crit">
        Danger zone
      </h2>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium text-ink-strong">Delete this project</p>
          <p
            class="text-sm text-ink-muted"
            data-testid="project-delete-summary">
            {{ projectDeleteSummary }}
          </p>
        </div>
        <DbButton
          variant="danger"
          data-testid="delete-project"
          @click="controller.startDelete({ kind: 'project' })">
          Delete project
        </DbButton>
      </div>
    </section>

    <NameDialog
      :open="dialog !== null"
      :title="dialogCopy.title"
      label="Environment name"
      :initial-name="dialogCopy.initialName"
      :submit-label="dialogCopy.submitLabel"
      :hint="dialogCopy.hint"
      placeholder="e.g. production"
      :action="controller.submitName"
      @close="dialog = null" />

    <DbConfirmDialog
      :open="deleting !== null"
      :title="`Delete ${deleteName}?`"
      :description="
        deleting?.kind === 'environment'
          ? `This can't be undone. Apps using ${deleteName}'s runner tokens stop working right away.`
          : `This can't be undone. Every environment, secret and runner token in ${deleteName} goes with it.`
      "
      :confirm-word="deleteName"
      :confirm-label="
        deleting?.kind === 'environment'
          ? 'Delete environment'
          : 'Delete project'
      "
      :affected-counts="deleteCounts"
      :loading="deleteLoading"
      :error="deleteError"
      @confirm="controller.confirmDelete"
      @close="controller.cancelDelete" />
  </div>
</template>
