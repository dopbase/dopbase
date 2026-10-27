<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { DbAlert, DbButton, DbInput, DbModal, DbSelect } from "~/components/ui";
import type { Workspace } from "~/services/workspaces.api";
import { DEFAULT_WORKSPACE_ID, workspaceLabel } from "~/constants/workspaces";

const props = defineProps<{
  open: boolean;
  title: string;
  kind: "workspace" | "project";
  workspaceMode?: "create" | "edit";
  initialName?: string;
  initialPath?: string;
  initialWorkspaceId?: string;
  workspaces?: Workspace[];
  action: (name: string, path: string, workspaceId: string) => Promise<void>;
}>();
const emit = defineEmits<{ close: [] }>();
const name = ref("");
const path = ref("");
const workspaceId = ref("");
const saving = ref(false);
const error = ref<string | null>(null);

const showWorkspaceRoot = computed(
  () => props.kind === "workspace" && props.workspaceMode === "edit",
);
const showProjectPath = computed(
  () => props.kind === "project" && Boolean(workspaceId.value),
);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    name.value = props.initialName ?? "";
    path.value = props.initialPath ?? "";
    workspaceId.value = props.initialWorkspaceId ?? DEFAULT_WORKSPACE_ID;
    error.value = null;
  },
  { immediate: true },
);

async function submit(): Promise<void> {
  saving.value = true;
  error.value = null;
  try {
    await props.action(
      name.value.trim(),
      path.value.trim(),
      workspaceId.value,
    );
    emit("close");
  } catch (cause) {
    error.value =
      cause instanceof Error ? cause.message : "Could not save the directory.";
  } finally {
    saving.value = false;
  }
}

const submitDisabled = computed(() => {
  if (props.kind === "workspace") {
    return !name.value.trim();
  }
  return Boolean(workspaceId.value) && !path.value.trim();
});

const helpText = computed(() => {
  if (props.kind === "workspace" && props.workspaceMode === "create") {
    return "Directory mapping for the CLI is configured on each machine (see workspace paths in the docs).";
  }
  if (props.kind === "workspace") {
    return "The root path is on the machine where you run the CLI.";
  }
  return "The CLI selects the most specific project directory containing its current working directory.";
});
</script>
<template>
  <DbModal :open="open" :title="title" :persistent="saving" @close="emit('close')">
    <form class="flex flex-col gap-4" @submit.prevent="submit">
      <DbInput
        v-if="kind === 'workspace'"
        v-model="name"
        label="Workspace name"
        placeholder="e.g. allior"
        :disabled="saving"
        autofocus />
      <DbSelect
        v-else
        v-model="workspaceId"
        label="Workspace"
        :disabled="saving"
        :options="
          (workspaces ?? []).map((item) => ({
            value: item.id,
            label: workspaceLabel(item),
          }))
        " />
      <DbInput
        v-if="showWorkspaceRoot || showProjectPath"
        v-model="path"
        :label="
          kind === 'workspace'
            ? 'Root directory'
            : 'Project directory (relative to workspace)'
        "
        :placeholder="
          kind === 'workspace'
            ? 'D:/Programming/Allior or /home/me/projects'
            : 'services/api or .'
        "
        :disabled="saving" />
      <p class="text-xs text-ink-muted">{{ helpText }}</p>
      <DbAlert v-if="error" variant="error">{{ error }}</DbAlert>
      <div class="flex justify-end gap-2">
        <DbButton :disabled="saving" @click="emit('close')">Cancel</DbButton>
        <DbButton
          type="submit"
          variant="primary"
          :loading="saving"
          :disabled="submitDisabled">
          Save
        </DbButton>
      </div>
    </form>
  </DbModal>
</template>
