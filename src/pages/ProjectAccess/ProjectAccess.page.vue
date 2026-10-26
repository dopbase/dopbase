<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  ENVIRONMENT_LEVELS,
  LEVEL_HINTS,
  PROJECT_LEVELS,
  WHOLE_PROJECT,
  useProjectAccessController,
} from "./ProjectAccess.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbEmptyState,
  DbModal,
  DbSelect,
  DbSkeleton,
  DbSwitch,
} from "~/components/ui";
import { ArrowLeftIcon, PlusIcon, ShieldIcon } from "~/assets/icons";
import { errorMessage } from "~/services/api-errors";
import { reaches } from "~/services/access.api";
import type { Grant, Level } from "~/services/access.api";
import { formatLevel } from "~/utils/format";

/**
 * ProjectAccess lists who reaches a project and lets its project managers
 * change that: grants on the whole project or one environment, for a user
 * or a group, and which environments are protected.
 */
const controller = useProjectAccessController();
const {
  access,
  grants,
  loading,
  loadError,
  actionError,
  principals,
  projectRef,
} = controller;

const managesProject = computed(
  () => access.value?.project.level === "project_manager",
);

const showGive = ref(false);
const principalKey = ref("");
const scope = ref(WHOLE_PROJECT);
const level = ref<Level>("read");
const giving = ref(false);
const giveError = ref<string | null>(null);
const removeTarget = ref<Grant | null>(null);
const removing = ref(false);

const principalOptions = computed(() =>
  principals.value.map((principal) => ({
    value: `${principal.type}:${principal.id}`,
    label:
      principal.type === "group"
        ? `${principal.label} (group)`
        : principal.label,
  })),
);
const scopeOptions = computed(() => [
  { value: WHOLE_PROJECT, label: "Whole project" },
  ...(access.value?.environments ?? []).map((environment) => ({
    value: environment.id,
    label: environment.protected
      ? `${environment.name} (protected)`
      : environment.name,
  })),
]);
const levelOptions = computed(() =>
  (scope.value === WHOLE_PROJECT ? PROJECT_LEVELS : ENVIRONMENT_LEVELS).map(
    (value) => ({
      value,
      label: `${formatLevel(value)}: ${LEVEL_HINTS[value]}`,
    }),
  ),
);

// Keep the level valid when the scope changes what is allowed.
watch(scope, () => {
  if (!levelOptions.value.some((option) => option.value === level.value)) {
    level.value = "read";
  }
});

function openGive(): void {
  principalKey.value = principalOptions.value[0]?.value ?? "";
  scope.value = WHOLE_PROJECT;
  level.value = "read";
  giveError.value = null;
  showGive.value = true;
}

async function submitGive(): Promise<void> {
  const principal = principals.value.find(
    (candidate) => `${candidate.type}:${candidate.id}` === principalKey.value,
  );
  if (!principal) {
    giveError.value = "Choose who gets access.";
    return;
  }
  giving.value = true;
  giveError.value = null;
  try {
    await controller.give(principal, scope.value, level.value);
    showGive.value = false;
  } catch (cause) {
    giveError.value = errorMessage(cause, "Could not give that access.");
  } finally {
    giving.value = false;
  }
}

async function confirmRemove(): Promise<void> {
  if (!removeTarget.value) return;
  removing.value = true;
  await controller.remove(removeTarget.value);
  removing.value = false;
  removeTarget.value = null;
}

function scopeName(grant: Grant): string {
  if (grant.scopeType === "project") return "Whole project";
  return grant.scopeLabel.split("/").slice(1).join("/") || grant.scopeLabel;
}
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-5xl px-4 py-6 sm:px-6 lg:p-8">
      <RouterLink
        :to="{ name: 'project', params: { projectRef } }"
        class="mb-4 inline-flex items-center gap-1.5 text-xs text-ink-muted transition-colors hover:text-ink-strong">
        <ArrowLeftIcon class="h-3.5 w-3.5" />
        Back to {{ projectRef }}
      </RouterLink>

      <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">
            Access to <span class="font-mono">{{ projectRef }}</span>
          </h1>
          <p class="mt-1 max-w-2xl text-sm text-ink-muted">
            A grant on one environment overrides a grant on the whole project.
            Protected environments ignore project grants below project manager.
            Root and people with access to every project aren't listed.
          </p>
        </div>
        <DbButton
          v-if="managesProject"
          size="sm"
          variant="primary"
          data-testid="give-access"
          @click="openGive">
          <PlusIcon class="h-3.5 w-3.5" />
          Give access
        </DbButton>
      </header>

      <DbAlert v-if="loadError" class="mb-4">{{ loadError }}</DbAlert>
      <DbAlert v-if="actionError" class="mb-4">{{ actionError }}</DbAlert>

      <div
        v-if="loading && !access"
        class="space-y-3"
        data-testid="access-skeleton">
        <DbSkeleton class="h-24 w-full rounded-card" />
        <DbSkeleton class="h-40 w-full rounded-card" />
      </div>

      <template v-else-if="access">
        <section class="mb-8">
          <h2 class="mb-2 text-sm font-medium text-ink-strong">Environments</h2>
          <div
            class="overflow-hidden rounded-card border border-line bg-panel"
            data-testid="access-environments">
            <div
              v-for="environment in access.environments"
              :key="environment.id"
              class="flex items-center gap-3 border-b border-line-soft px-4 py-3 last:border-0">
              <span class="min-w-0 flex-1 truncate font-mono text-sm">
                {{ environment.name }}
              </span>
              <DbSwitch
                :model-value="environment.protected ?? false"
                label="Protected"
                :disabled="!reaches(environment.level, 'manage')"
                @update:model-value="
                  (value: boolean) =>
                    controller.setProtected(environment.id, value)
                " />
            </div>
            <p
              v-if="access.environments.length === 0"
              class="px-4 py-3 text-sm text-ink-muted">
              This project has no environments yet.
            </p>
          </div>
        </section>

        <section>
          <h2 class="mb-2 text-sm font-medium text-ink-strong">
            Who has access
          </h2>
          <DbEmptyState
            v-if="grants.length === 0"
            title="No grants on this project"
            description="Only root and people with access to every project reach it. Give a user or a group access to share it.">
            <template #icon>
              <ShieldIcon class="h-5 w-5" />
            </template>
          </DbEmptyState>
          <div
            v-else
            class="overflow-x-auto rounded-card border border-line bg-panel">
            <table
              class="min-w-full text-left text-sm"
              data-testid="access-grants">
              <thead>
                <tr class="border-b border-line">
                  <th class="px-4 py-2.5 text-xs font-medium text-ink-muted">
                    Who
                  </th>
                  <th class="px-4 py-2.5 text-xs font-medium text-ink-muted">
                    Where
                  </th>
                  <th class="px-4 py-2.5 text-xs font-medium text-ink-muted">
                    Level
                  </th>
                  <th class="px-4 py-2.5" />
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="grant in grants"
                  :key="grant.id"
                  class="border-b border-line-soft last:border-0">
                  <td class="px-4 py-2.5">
                    <span class="font-mono text-sm text-ink-strong">
                      {{ grant.principalLabel }}
                    </span>
                    <DbBadge
                      v-if="grant.principalType === 'group'"
                      class="ml-2">
                      group
                    </DbBadge>
                  </td>
                  <td class="px-4 py-2.5 font-mono text-xs text-ink-muted">
                    {{ scopeName(grant) }}
                  </td>
                  <td class="px-4 py-2.5">
                    <DbBadge
                      :tone="grant.level === 'none' ? 'crit' : 'neutral'">
                      {{ formatLevel(grant.level) }}
                    </DbBadge>
                  </td>
                  <td class="px-4 py-2.5 text-right">
                    <DbButton
                      v-if="managesProject"
                      size="sm"
                      variant="ghost"
                      :aria-label="`Remove access for ${grant.principalLabel}`"
                      @click="removeTarget = grant">
                      Remove
                    </DbButton>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </template>
    </div>

    <DbModal :open="showGive" title="Give access" @close="showGive = false">
      <form class="flex flex-col gap-4" @submit.prevent="submitGive">
        <DbSelect
          v-if="principalOptions.length > 0"
          v-model="principalKey"
          label="User or group"
          :options="principalOptions" />
        <DbAlert v-else tone="info">
          There is nobody you can list. Ask someone who can see users and groups
          to give access.
        </DbAlert>
        <DbSelect v-model="scope" label="Where" :options="scopeOptions" />
        <DbSelect
          :model-value="level"
          label="Level"
          :options="levelOptions"
          @update:model-value="(value: string) => (level = value as Level)" />
        <p class="text-xs text-ink-muted">
          Giving access to someone who already has a grant here replaces it.
        </p>
        <p v-if="giveError" class="text-xs text-crit">{{ giveError }}</p>
        <div class="flex justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="giving"
            @click="showGive = false">
            Cancel
          </DbButton>
          <DbButton
            variant="primary"
            type="submit"
            :loading="giving"
            :disabled="principalOptions.length === 0">
            Give access
          </DbButton>
        </div>
      </form>
    </DbModal>

    <DbConfirmDialog
      :open="removeTarget !== null"
      title="Remove access"
      :description="`${removeTarget?.principalLabel} loses this grant on their next request.`"
      confirm-label="Remove"
      :loading="removing"
      @confirm="confirmRemove"
      @close="removeTarget = null" />
  </DashboardLayout>
</template>
