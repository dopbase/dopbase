<script setup lang="ts">
import { computed, ref } from "vue";
import {
  useAccessPanelController,
  LEVEL_HINTS,
} from "./AccessPanel.controller";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbEmptyState,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbModal,
  DbRadioCard,
  DbSearchField,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import {
  InfoIcon,
  LockIcon,
  PlusIcon,
  ShieldIcon,
  SlidersHorizontalIcon,
  UsersIcon,
  XIcon,
} from "~/assets/icons";
import type { Grant } from "~/services/access.api";

/**
 * AccessPanel — the project's Access tab: who reaches the project and at
 * what level, on the whole project or one environment. Project managers
 * give, change and remove access; people with access.report only read.
 */
const controller = useAccessPanelController();
const {
  access,
  rows,
  filter,
  canManage,
  loading,
  loadError,
  actionError,
  principals,
  dialog,
  principalKey,
  where,
  environmentId,
  level,
  levels,
  saving,
  dialogError,
  dialogSummary,
} = controller;

const removeTarget = ref<Grant | null>(null);
const removing = ref(false);

async function confirmRemove(): Promise<void> {
  if (!removeTarget.value) return;
  removing.value = true;
  await controller.remove(removeTarget.value);
  removing.value = false;
  removeTarget.value = null;
}

const principalOptions = computed(() =>
  principals.value.map((principal) => ({
    value: `${principal.type}:${principal.id}`,
    label:
      principal.type === "group"
        ? `${principal.label} (group)`
        : principal.label,
  })),
);
const whereOptions = [
  { value: "project", label: "Whole project" },
  { value: "environment", label: "One environment" },
];
const environmentOptions = computed(() =>
  (access.value?.environments ?? []).map((environment) => ({
    value: environment.id,
    label: environment.protected
      ? `${environment.name} (protected)`
      : environment.name,
  })),
);
const dialogTitle = computed(() =>
  dialog.value?.mode === "change"
    ? `Change access for ${dialog.value.grant.principalLabel}`
    : `Give access to ${access.value?.project.name ?? "this project"}`,
);
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-wrap items-center gap-2">
      <DbSearchField
        v-model="filter"
        class="w-full sm:w-[232px]"
        placeholder="Filter people and groups" />
      <DbButton
        v-if="canManage"
        class="ml-auto"
        variant="primary"
        data-testid="give-access"
        @click="controller.openGive">
        <PlusIcon class="h-4 w-4" />
        Give access
      </DbButton>
    </div>

    <DbAlert v-if="loadError">{{ loadError }}</DbAlert>
    <DbAlert v-if="actionError">{{ actionError }}</DbAlert>

    <div
      v-if="loading && !access"
      class="flex flex-col gap-3"
      data-testid="access-skeleton">
      <DbSkeleton class="h-40 w-full rounded-card" />
    </div>

    <template v-else-if="access">
      <DbEmptyState
        v-if="rows.length === 0 && !filter.trim()"
        title="No one has been given access yet"
        description="Only root and people with access to every project reach it. Give a person or a group access to share it.">
        <template #icon>
          <ShieldIcon class="h-5 w-5" />
        </template>
      </DbEmptyState>

      <div
        v-else
        class="overflow-x-auto rounded-card border border-line bg-panel">
        <table class="min-w-full text-left text-sm" data-testid="access-grants">
          <thead>
            <tr class="border-b border-line-soft">
              <th class="h-11 px-4 font-medium text-ink-muted">Who</th>
              <th
                class="hidden h-11 px-4 font-medium text-ink-muted sm:table-cell">
                Where
              </th>
              <th class="h-11 px-4 font-medium text-ink-muted">Level</th>
              <th class="h-11 w-12 px-2">
                <span class="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="rows.length === 0">
              <td colspan="4" class="px-4 py-8 text-center text-ink-muted">
                Nobody matches “{{ filter.trim() }}”.
              </td>
            </tr>
            <tr
              v-for="row in rows"
              :key="row.grant.id"
              class="border-b border-line-soft transition-colors last:border-b-0 hover:bg-raised"
              data-testid="access-row">
              <td class="h-12 w-full max-w-0 px-4 sm:w-[40%]">
                <div class="flex min-w-0 items-center gap-2">
                  <UsersIcon
                    v-if="row.isGroup"
                    class="h-4 w-4 shrink-0 text-accent-strong" />
                  <span
                    class="truncate font-mono text-sm text-ink-strong"
                    :title="row.grant.principalLabel">
                    {{ row.grant.principalLabel }}
                  </span>
                  <DbBadge v-if="row.isGroup" tone="accent">group</DbBadge>
                </div>
              </td>
              <td class="hidden h-12 px-4 sm:table-cell">
                <span v-if="row.where === 'Whole project'" class="text-ink">
                  Whole project
                </span>
                <span
                  v-else
                  class="inline-flex items-center gap-1.5 rounded-control border border-line px-2 py-0.5 font-mono text-sm text-ink-strong">
                  {{ row.where }}
                  <LockIcon
                    v-if="row.whereProtected"
                    class="h-3.5 w-3.5 text-warn"
                    aria-label="protected" />
                </span>
              </td>
              <td class="h-12 whitespace-nowrap px-4">
                <DbLevelBadge v-bind="row.meter" />
              </td>
              <td class="h-12 px-2 text-right">
                <DbMenu
                  v-if="canManage"
                  :label="`Actions for ${row.grant.principalLabel}`">
                  <DbMenuItem @select="controller.openChange(row.grant)">
                    <template #icon>
                      <SlidersHorizontalIcon class="h-4 w-4" />
                    </template>
                    Change level
                  </DbMenuItem>
                  <div class="my-0.5 h-px bg-line-soft" role="separator" />
                  <DbMenuItem tone="danger" @select="removeTarget = row.grant">
                    <template #icon><XIcon class="h-4 w-4" /></template>
                    Remove access
                  </DbMenuItem>
                </DbMenu>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <p class="flex items-start gap-2 text-sm text-ink-muted">
        <InfoIcon class="mt-0.5 h-4 w-4 shrink-0" />
        A grant on one environment overrides a grant on the whole project. Root
        and people with access to every project aren't listed.
      </p>
    </template>

    <!-- Give access / change level -->
    <DbModal
      :open="dialog !== null"
      :title="dialogTitle"
      @close="!saving && (dialog = null)">
      <form
        id="access-dialog"
        class="flex flex-col gap-4"
        @submit.prevent="controller.submitDialog">
        <template v-if="dialog?.mode === 'give'">
          <DbSelect
            v-if="principalOptions.length > 0"
            v-model="principalKey"
            label="User or group"
            :options="principalOptions" />
          <DbAlert v-else tone="info">
            There's nobody you can list. Ask someone who can see users and
            groups to give access.
          </DbAlert>
          <div class="grid gap-3 sm:grid-cols-2">
            <DbSelect v-model="where" label="Where" :options="whereOptions" />
            <DbSelect
              v-if="where === 'environment'"
              v-model="environmentId"
              label="Environment"
              :options="environmentOptions" />
          </div>
        </template>

        <fieldset class="flex flex-col gap-2">
          <legend class="mb-1.5 text-sm text-ink-muted">Level</legend>
          <DbRadioCard
            v-for="candidate in levels"
            :key="candidate"
            name="access-level"
            :value="candidate"
            :checked="level === candidate"
            :label="controller.formatLevel(candidate)"
            :description="LEVEL_HINTS[candidate]"
            @select="level = candidate" />
        </fieldset>

        <p
          v-if="dialogSummary"
          class="flex items-start gap-2.5 rounded-control bg-raised px-3.5 py-3 text-sm text-ink"
          data-testid="access-summary">
          <InfoIcon class="mt-0.5 h-4 w-4 shrink-0 text-accent-strong" />
          {{ dialogSummary }}
        </p>
        <DbAlert v-if="dialogError">{{ dialogError }}</DbAlert>
      </form>
      <template #footer>
        <DbButton variant="ghost" :disabled="saving" @click="dialog = null">
          Cancel
        </DbButton>
        <DbButton
          variant="primary"
          type="submit"
          form="access-dialog"
          :loading="saving">
          {{ dialog?.mode === "change" ? "Save level" : "Give access" }}
        </DbButton>
      </template>
    </DbModal>

    <DbConfirmDialog
      :open="removeTarget !== null"
      title="Remove access"
      :description="`${removeTarget?.principalLabel} loses this access on their next request.`"
      confirm-label="Remove access"
      tone="danger"
      :loading="removing"
      @confirm="confirmRemove"
      @close="removeTarget = null" />
  </div>
</template>
