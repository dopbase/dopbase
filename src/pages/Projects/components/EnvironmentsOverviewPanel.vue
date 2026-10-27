<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { ProjectsController } from "../Projects.controller";
import { useEnvironmentsOverviewController } from "./EnvironmentsOverview.controller";
import EnvFileEditor from "./EnvFileEditor.vue";
import {
  DbAlert,
  DbButton,
  DbEmptyState,
  DbInput,
  DbSkeleton,
} from "~/components/ui";
import { KeyIcon, LayersIcon } from "~/assets/icons";

const props = defineProps<{
  controller: ProjectsController;
  projectName: string;
}>();

const projectRef = computed(() => props.controller.projectRef.value);
const environments = computed(() => props.controller.environments.value);
const overview = useEnvironmentsOverviewController(projectRef, environments);

type PanelView = "table" | "editor";
const view = ref<PanelView>("table");

watch(
  () => props.controller.isProjectOverview.value,
  (open) => {
    if (!open) {
      view.value = "table";
      overview.closeEditors();
    }
  },
);

function switchView(next: PanelView): void {
  if (next === view.value) return;
  if (next === "editor") {
    view.value = "editor";
    void overview.openEditors();
    return;
  }
  overview.closeEditors();
  view.value = "table";
}

const draftValues = ref<Record<string, string>>({});

function draftKey(environmentId: string, key: string): string {
  return `${environmentId}:${key}`;
}

function openKey(key: string): void {
  overview.expandedKey.value =
    overview.expandedKey.value === key ? null : key;
  draftValues.value = {};
}

async function saveKey(key: string): Promise<void> {
  for (const row of overview.rows.value) {
    const field = draftKey(row.environment.id, key);
    const value = draftValues.value[field];
    if (value === undefined) continue;
    await overview.setSecret(row.environment.id, key, value);
  }
  overview.expandedKey.value = null;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <header class="flex flex-wrap items-center gap-2">
      <h3 class="text-sm font-semibold">
        All environments
        <span class="ml-1 font-mono text-xs text-ink-muted">
          {{ projectName }}
        </span>
      </h3>
      <div class="ml-auto flex items-center gap-2">
        <div
          class="flex rounded-control border border-line bg-canvas p-0.5"
          role="tablist"
          aria-label="Environments view">
          <button
            type="button"
            role="tab"
            :aria-selected="view === 'table'"
            class="cursor-pointer rounded px-2.5 py-1 text-xs font-medium transition-colors"
            :class="
              view === 'table'
                ? 'bg-raised text-ink-strong'
                : 'text-ink-muted hover:text-ink'
            "
            @click="switchView('table')">
            Keys
          </button>
          <button
            type="button"
            role="tab"
            :aria-selected="view === 'editor'"
            class="cursor-pointer rounded px-2.5 py-1 text-xs font-medium transition-colors"
            :class="
              view === 'editor'
                ? 'bg-raised text-ink-strong'
                : 'text-ink-muted hover:text-ink'
            "
            @click="switchView('editor')">
            Edit as .env
          </button>
        </div>
        <DbButton
          v-if="view === 'editor' && overview.editorDirty.value"
          size="sm"
          variant="primary"
          :loading="overview.editorSaving.value"
          @click="overview.saveEditors()">
          Save all
        </DbButton>
      </div>
    </header>

    <DbAlert v-if="overview.actionError.value">
      {{ overview.actionError.value }}
    </DbAlert>

    <template v-if="view === 'table'">
      <div
        v-if="overview.loading.value"
        class="flex flex-col gap-2"
        data-testid="env-overview-skeleton">
        <DbSkeleton class="h-10 w-full rounded-card" />
        <DbSkeleton class="h-10 w-full rounded-card" />
        <DbSkeleton class="h-10 w-full rounded-card" />
      </div>
      <DbAlert v-else-if="overview.loadError.value">
        {{ overview.loadError.value }}
      </DbAlert>
      <DbEmptyState
        v-else-if="!environments?.length"
        title="No environments"
        description="Create an environment from the rail to start storing secrets.">
        <template #icon>
          <LayersIcon class="h-5 w-5" />
        </template>
      </DbEmptyState>
      <div
        v-else
        class="overflow-x-auto rounded-card border border-line bg-panel">
        <table class="min-w-full text-left text-sm">
          <thead>
            <tr class="border-b border-line text-xs uppercase tracking-wide text-ink-faint">
              <th class="px-4 py-2.5 font-medium">Key</th>
              <th
                v-for="row in overview.rows.value"
                :key="row.environment.id"
                class="px-4 py-2.5 text-center font-medium">
                <button
                  type="button"
                  class="cursor-pointer font-mono text-ink hover:text-ink-strong"
                  @click="
                    controller.selectEnvironment(row.environment.id)
                  ">
                  {{ row.environment.name }}
                </button>
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="key in overview.allKeys.value" :key="key">
              <tr class="border-b border-line-soft hover:bg-raised/40">
                <td class="px-4 py-2">
                  <button
                    type="button"
                    class="flex cursor-pointer items-center gap-2 font-mono text-xs text-ink-strong"
                    @click="openKey(key)">
                    <KeyIcon class="h-3.5 w-3.5 text-ink-faint" />
                    {{ key }}
                  </button>
                </td>
                <td
                  v-for="row in overview.rows.value"
                  :key="`${key}-${row.environment.id}`"
                  class="px-4 py-2 text-center">
                  <span
                    class="inline-block h-2.5 w-2.5 rounded-full"
                    :class="
                      overview.hasKey(key, row.environment.id)
                        ? 'bg-ok'
                        : 'border border-line bg-transparent'
                    "
                    :title="
                      overview.hasKey(key, row.environment.id)
                        ? 'Set'
                        : 'Missing'
                    " />
                </td>
              </tr>
              <tr
                v-if="overview.expandedKey.value === key"
                class="border-b border-line bg-canvas">
                <td :colspan="overview.rows.value.length + 1" class="px-4 py-4">
                  <div class="flex flex-col gap-3">
                    <div
                      v-for="row in overview.rows.value"
                      :key="`${key}-edit-${row.environment.id}`"
                      class="grid gap-2 rounded-control border border-line bg-panel p-3 md:grid-cols-[8rem_1fr_auto] md:items-center">
                      <span class="font-mono text-xs text-ink-muted">
                        {{ row.environment.name }}
                      </span>
                      <DbInput
                        :model-value="
                          draftValues[draftKey(row.environment.id, key)] ?? ''
                        "
                        placeholder="Value"
                        @update:model-value="
                          (value) =>
                            (draftValues[draftKey(row.environment.id, key)] =
                              value)
                        " />
                      <DbButton
                        v-if="overview.hasKey(key, row.environment.id)"
                        size="sm"
                        variant="secondary"
                        @click="
                          overview.deleteSecret(row.environment.id, key)
                        ">
                        Remove
                      </DbButton>
                    </div>
                    <div class="flex justify-end gap-2">
                      <DbButton
                        size="sm"
                        @click="overview.expandedKey.value = null">
                        Cancel
                      </DbButton>
                      <DbButton
                        size="sm"
                        variant="primary"
                        @click="saveKey(key)">
                        Save key
                      </DbButton>
                    </div>
                  </div>
                </td>
              </tr>
            </template>
            <tr v-if="overview.allKeys.value.length === 0">
              <td
                :colspan="(overview.rows.value.length || 1) + 1"
                class="px-4 py-8 text-center text-sm text-ink-muted">
                No secrets yet. Open an environment or add values from a row.
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>

    <template v-else>
      <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden">
        <DbAlert v-if="overview.editorAwaitingReauth.value" tone="info">
          Confirm your password to load secret values for editing.
        </DbAlert>
        <DbAlert v-else-if="overview.editorLoadError.value">
          {{ overview.editorLoadError.value }}
        </DbAlert>
        <div
          v-else-if="overview.editorLoading.value"
          class="flex min-h-0 flex-1 gap-3">
          <DbSkeleton
            v-for="row in environments ?? []"
            :key="row.id"
            class="min-h-0 flex-1 rounded-card" />
        </div>
        <DbAlert v-else-if="overview.editorError.value">
          {{ overview.editorError.value }}
        </DbAlert>
        <div
          v-else
          class="flex min-h-0 flex-1 gap-3 overflow-hidden"
          data-testid="multi-env-editor">
          <div
            v-for="editor in overview.editors.value"
            :key="editor.environmentId"
            class="flex min-h-0 min-w-0 flex-1 flex-col">
            <EnvFileEditor
              fill-height
              :model-value="editor.content"
              :issues="editor.issues"
              :disabled="overview.editorSaving.value || editor.loading"
              :subtitle="editor.environmentName"
              :dirty="editor.content !== editor.baseline"
              :peer-active-line="overview.sharedCaretLine.value"
              @update:model-value="
                (value) =>
                  overview.updateEditorContent(editor.environmentId, value)
              "
              @caret-line="(line) => (overview.sharedCaretLine.value = line)"
              @save="overview.saveEditors()" />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
