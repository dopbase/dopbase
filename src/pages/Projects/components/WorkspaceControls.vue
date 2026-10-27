<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ProjectsController } from "../Projects.controller";
import { DbConfirmDialog, DbModal, DbSelect } from "~/components/ui";
import DirectoryDialog from "./DirectoryDialog.vue";
import WorkspaceTokensPanel from "./WorkspaceTokensPanel.vue";
import { workspaceLabel } from "~/constants/workspaces";
import { useAuthStore } from "~/stores/auth.store";

const props = defineProps<{ controller: ProjectsController }>();
const emit = defineEmits<{ dismissRail: [] }>();

const auth = useAuthStore();

const options = computed(() =>
  (props.controller.workspaces?.value ?? []).map((item) => ({
    value: item.id,
    label: workspaceLabel(item),
  })),
);

const dialog = ref<"create" | "edit" | null>(null);
const tokensOpen = ref(false);
const deleting = ref(false);
const busy = ref(false);
const error = ref<string | null>(null);
const menuOpen = ref(false);
const menuRoot = ref<HTMLElement | null>(null);

const iconButtonClass =
  "cursor-pointer rounded-control bg-raised p-1 text-ink-muted transition-colors hover:bg-line hover:text-ink-strong disabled:cursor-default disabled:opacity-50";

function dismissRail(): void {
  emit("dismissRail");
}

function closeMenu(): void {
  menuOpen.value = false;
}

function resetDialogs(): void {
  dialog.value = null;
  tokensOpen.value = false;
  deleting.value = false;
  closeMenu();
}

function openCreateWorkspace(): void {
  closeMenu();
  dismissRail();
  dialog.value = "create";
}

function openEditWorkspace(): void {
  closeMenu();
  dismissRail();
  dialog.value = "edit";
}

function openWorkspaceTokens(): void {
  closeMenu();
  dismissRail();
  tokensOpen.value = true;
}

function openDeleteWorkspace(): void {
  closeMenu();
  dismissRail();
  deleting.value = true;
}

function onDocumentPointerDown(event: PointerEvent): void {
  if (!menuOpen.value || !menuRoot.value) return;
  if (!menuRoot.value.contains(event.target as Node)) {
    closeMenu();
  }
}

watch(
  () => auth.sessionExpired,
  (expired) => {
    if (expired) resetDialogs();
  },
);

onMounted(() => {
  document.addEventListener("pointerdown", onDocumentPointerDown);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onDocumentPointerDown);
});

async function remove(): Promise<void> {
  const workspace = props.controller.workspace.value;
  if (!workspace) return;
  busy.value = true;
  error.value = null;
  try {
    await props.controller.deleteWorkspace(workspace.id);
    deleting.value = false;
  } catch (cause) {
    error.value =
      cause instanceof Error ? cause.message : "Could not delete workspace.";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="shrink-0 border-b border-line-soft">
    <div
      class="flex h-[4.75rem] min-h-[4.75rem] items-center gap-2 px-4 py-4 md:gap-2.5 md:px-5"
      data-testid="workspace-toolbar">
      <DbSelect
        compact
        class="min-w-0 flex-1"
        label="Workspace"
        :model-value="controller.workspaceId?.value"
        :options="options"
        @update:model-value="controller.selectWorkspace" />
      <div ref="menuRoot" class="relative shrink-0">
        <button
          type="button"
          :class="iconButtonClass"
          aria-label="Workspace actions"
          aria-haspopup="menu"
          :aria-expanded="menuOpen"
          data-testid="workspace-actions"
          @click="menuOpen = !menuOpen">
          <span class="flex flex-col items-center gap-0.5 px-0.5" aria-hidden="true">
            <span class="block h-0.5 w-0.5 rounded-full bg-current" />
            <span class="block h-0.5 w-0.5 rounded-full bg-current" />
            <span class="block h-0.5 w-0.5 rounded-full bg-current" />
          </span>
        </button>
        <div
          v-if="menuOpen"
          role="menu"
          class="absolute right-0 top-full z-20 mt-1 min-w-[11rem] overflow-hidden rounded-control border border-line bg-panel py-1 shadow-lg">
          <button
            type="button"
            role="menuitem"
            class="flex w-full cursor-pointer px-3 py-2 text-left text-sm text-ink hover:bg-raised"
            data-testid="new-workspace"
            @click="openCreateWorkspace">
            New workspace
          </button>
          <template v-if="controller.workspace?.value">
            <button
              type="button"
              role="menuitem"
              class="flex w-full cursor-pointer px-3 py-2 text-left text-sm text-ink hover:bg-raised"
              data-testid="workspace-tokens"
              @click="openWorkspaceTokens">
              Workspace tokens
            </button>
            <button
              type="button"
              role="menuitem"
              class="flex w-full cursor-pointer px-3 py-2 text-left text-sm text-ink hover:bg-raised"
              @click="openEditWorkspace">
              Workspace settings
            </button>
            <button
              type="button"
              role="menuitem"
              class="flex w-full cursor-pointer px-3 py-2 text-left text-sm text-crit hover:bg-crit/10"
              @click="openDeleteWorkspace">
              Delete workspace
            </button>
          </template>
        </div>
      </div>
    </div>
    <p
      v-if="controller.workspacesError?.value"
      class="px-4 pb-3 text-xs text-crit md:px-5 md:pb-4 md:pt-0">
      {{ controller.workspacesError.value }}
    </p>
    <p
      v-else-if="controller.workspace?.value"
      class="break-all px-4 pb-3 font-mono text-xs text-ink-faint md:hidden">
      {{ controller.workspace.value.rootPath }}
    </p>
  </div>
  <DirectoryDialog
    :open="dialog !== null"
    kind="workspace"
    :workspace-mode="dialog === 'edit' ? 'edit' : 'create'"
    :title="dialog === 'edit' ? 'Workspace settings' : 'New workspace'"
    :initial-name="dialog === 'edit' ? controller.workspace.value?.name : ''"
    :initial-path="dialog === 'edit' ? controller.workspace.value?.rootPath : ''"
    :action="
      (name, rootPath) =>
        dialog === 'edit' && controller.workspace.value
          ? controller.updateWorkspace(controller.workspace.value.id, {
              name,
              rootPath,
            })
          : controller.createWorkspace({ name })
    "
    @close="dialog = null" />
  <DbModal
    :open="tokensOpen && !!controller.workspace?.value"
    title="Workspace tokens"
    size="lg"
    @close="tokensOpen = false">
    <WorkspaceTokensPanel
      v-if="controller.workspace?.value"
      :workspace-id="controller.workspace.value.id" />
  </DbModal>
  <DbConfirmDialog
    :open="deleting"
    title="Delete workspace"
    description="Only an empty workspace can be deleted. Move its projects to another workspace first."
    :confirm-word="controller.workspace?.value?.name"
    :loading="busy"
    :error="error"
    @confirm="remove"
    @close="deleting = false" />
</template>
