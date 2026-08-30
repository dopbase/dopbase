<script setup lang="ts">
import { ref } from "vue";
import type { WorkspaceController } from "~/pages/Workspace/Workspace.controller";
import NameDialog from "./NameDialog.vue";
import { DbConfirmDialog, DbSpinner } from "~/components/ui";
import {
  BoxIcon,
  LayersIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
} from "~/assets/icons";
import type { Environment, Project } from "~/services";

/**
 * ProjectRail — the project/environment keyline rail.
 *
 * Renders the hierarchy and selection state plus all project/environment
 * create, rename, and delete interactions. Destructive dialogs require
 * typing the resource name; environment deletion lists affected counts.
 */
const props = defineProps<{ controller: WorkspaceController }>();

// Destructure so refs auto-unwrap in the template.
const {
  projects,
  projectsError,
  environments,
  environmentsLoading,
  environmentsError,
} = props.controller;

type ProjectDialogState =
  { mode: "create" } | { mode: "rename"; project: Project } | null;
type EnvDialogState =
  { mode: "create" } | { mode: "rename"; environment: Environment } | null;

const projectDialog = ref<ProjectDialogState>(null);
const envDialog = ref<EnvDialogState>(null);
const projectDelete = ref<Project | null>(null);
const envDelete = ref<Environment | null>(null);
const deleteLoading = ref(false);
const deleteError = ref<string | null>(null);
const envDeleteCounts = ref<Array<{ label: string; count: number }>>([]);

const isProjectActive = (project: Project): boolean =>
  props.controller.projectRef.value === project.name ||
  props.controller.projectRef.value === project.id;
const isEnvActive = (environment: Environment): boolean =>
  props.controller.environmentId.value === environment.id;

async function openEnvDelete(environment: Environment): Promise<void> {
  envDelete.value = environment;
  deleteError.value = null;
  envDeleteCounts.value = [];
  try {
    envDeleteCounts.value = await props.controller.describeEnvironmentDeletion(
      environment.id,
    );
  } catch {
    // The preview is best-effort; deletion still confirms by name.
  }
}

async function confirmProjectDelete(): Promise<void> {
  if (!projectDelete.value) return;
  deleteLoading.value = true;
  deleteError.value = null;
  try {
    await props.controller.deleteProject();
    projectDelete.value = null;
  } catch {
    deleteError.value = "The project could not be deleted.";
  } finally {
    deleteLoading.value = false;
  }
}

async function confirmEnvDelete(): Promise<void> {
  if (!envDelete.value) return;
  deleteLoading.value = true;
  deleteError.value = null;
  try {
    await props.controller.deleteEnvironment(envDelete.value.id);
    envDelete.value = null;
  } catch {
    deleteError.value = "The environment could not be deleted.";
  } finally {
    deleteLoading.value = false;
  }
}
</script>

<template>
  <aside
    class="sticky top-0 flex h-svh w-72 shrink-0 flex-col border-r border-line bg-panel/60">
    <header class="flex items-center justify-between px-4 py-3.5">
      <h2 class="font-mono text-xs uppercase tracking-wider text-ink-faint">
        projects
      </h2>
      <button
        type="button"
        class="cursor-pointer rounded-md border border-line bg-raised p-1 text-ink-muted transition-colors hover:border-accent/50 hover:text-ink-strong"
        aria-label="New project"
        data-testid="new-project"
        @click="projectDialog = { mode: 'create' }">
        <PlusIcon class="h-4 w-4" />
      </button>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
      <DbSpinner v-if="!projects" class="mx-auto mt-8 h-5 w-5 text-ink-muted" />
      <p v-else-if="projectsError" class="px-2 py-4 text-xs text-crit">
        {{ projectsError }}
      </p>
      <ul v-else-if="projects.length > 0" class="flex flex-col gap-0.5">
        <li v-for="project in projects" :key="project.id">
          <!-- Project row -->
          <div
            class="group flex items-center gap-1 rounded-md px-2 py-1.5"
            :class="
              isProjectActive(project) ? 'bg-raised' : 'hover:bg-raised/60'
            ">
            <button
              type="button"
              class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
              @click="controller.selectProject(project.name)">
              <BoxIcon
                class="h-3.5 w-3.5 shrink-0"
                :class="
                  isProjectActive(project)
                    ? 'text-accent-strong'
                    : 'text-ink-faint'
                " />
              <span
                class="truncate font-mono text-xs"
                :class="
                  isProjectActive(project) ? 'text-ink-strong' : 'text-ink'
                ">
                {{ project.name }}
              </span>
            </button>
            <span
              class="flex shrink-0 items-center opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
              <button
                type="button"
                class="cursor-pointer rounded p-1 text-ink-faint hover:text-ink-strong"
                :aria-label="`Rename ${project.name}`"
                @click="projectDialog = { mode: 'rename', project }">
                <PencilIcon class="h-3 w-3" />
              </button>
              <button
                type="button"
                class="cursor-pointer rounded p-1 text-ink-faint hover:text-crit"
                :aria-label="`Delete ${project.name}`"
                @click="projectDelete = project">
                <TrashIcon class="h-3 w-3" />
              </button>
            </span>
          </div>

          <!-- Environments of the active project -->
          <ul
            v-if="isProjectActive(project)"
            class="mb-1 ml-4 flex flex-col gap-0.5 border-l border-line-soft pl-2 pt-0.5">
            <li v-for="environment in environments ?? []" :key="environment.id">
              <div
                class="group flex items-center gap-1 rounded-md px-2 py-1.5"
                :class="
                  isEnvActive(environment)
                    ? 'bg-accent-soft'
                    : 'hover:bg-raised/60'
                ">
                <button
                  type="button"
                  class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
                  @click="controller.selectEnvironment(environment.id)">
                  <LayersIcon
                    class="h-3 w-3 shrink-0"
                    :class="
                      isEnvActive(environment)
                        ? 'text-accent-strong'
                        : 'text-ink-faint'
                    " />
                  <span
                    class="truncate font-mono text-xs"
                    :class="
                      isEnvActive(environment)
                        ? 'text-ink-strong'
                        : 'text-ink-muted'
                    ">
                    {{ environment.name }}
                  </span>
                </button>
                <span
                  class="flex shrink-0 items-center opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                  <button
                    type="button"
                    class="cursor-pointer rounded p-1 text-ink-faint hover:text-ink-strong"
                    :aria-label="`Rename ${environment.name}`"
                    @click="envDialog = { mode: 'rename', environment }">
                    <PencilIcon class="h-3 w-3" />
                  </button>
                  <button
                    type="button"
                    class="cursor-pointer rounded p-1 text-ink-faint hover:text-crit"
                    :aria-label="`Delete ${environment.name}`"
                    @click="openEnvDelete(environment)">
                    <TrashIcon class="h-3 w-3" />
                  </button>
                </span>
              </div>
            </li>

            <li v-if="environmentsLoading" class="px-2 py-1.5">
              <DbSpinner class="h-3.5 w-3.5 text-ink-faint" />
            </li>
            <li
              v-else-if="environmentsError"
              class="px-2 py-1.5 text-xs text-crit">
              {{ environmentsError }}
            </li>

            <li>
              <button
                type="button"
                class="flex w-full cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 font-mono text-xs text-ink-faint transition-colors hover:bg-raised/60 hover:text-ink"
                @click="envDialog = { mode: 'create' }">
                <PlusIcon class="h-3 w-3" />
                new environment
              </button>
            </li>
          </ul>
        </li>
      </ul>

      <p v-else class="px-2 py-4 text-xs leading-relaxed text-ink-muted">
        No projects yet. Create one with the + button above.
      </p>
    </div>

    <!-- Create / rename project -->
    <NameDialog
      :open="projectDialog !== null"
      :title="
        projectDialog?.mode === 'rename' ? 'Rename project' : 'New project'
      "
      label="Project name"
      :initial-name="
        projectDialog?.mode === 'rename' ? projectDialog.project.name : ''
      "
      :submit-label="projectDialog?.mode === 'rename' ? 'Rename' : 'Create'"
      hint="Names are unique on this server. Example: payment-service"
      :action="
        (name: string) =>
          projectDialog?.mode === 'rename'
            ? controller.renameProject(name)
            : controller.createProject(name)
      "
      @close="projectDialog = null" />

    <!-- Create / rename environment -->
    <NameDialog
      :open="envDialog !== null"
      :title="
        envDialog?.mode === 'rename' ? 'Rename environment' : 'New environment'
      "
      label="Environment name"
      :initial-name="
        envDialog?.mode === 'rename' ? envDialog.environment.name : ''
      "
      :submit-label="envDialog?.mode === 'rename' ? 'Rename' : 'Create'"
      hint="Example: development, staging, production"
      placeholder="e.g. production, staging, development"
      :action="
        (name: string) =>
          envDialog?.mode === 'rename'
            ? controller.renameEnvironment(envDialog.environment.id, name)
            : controller.createEnvironment(name)
      "
      @close="envDialog = null" />

    <!-- Delete project -->
    <DbConfirmDialog
      :open="projectDelete !== null"
      title="Delete project"
      :description="`Deleting '${projectDelete?.name}' permanently removes it with all environments, secrets, and scoped runner tokens.`"
      :confirm-word="projectDelete?.name"
      confirm-label="Delete project"
      :loading="deleteLoading"
      :error="deleteError"
      @confirm="confirmProjectDelete"
      @close="projectDelete = null" />

    <!-- Delete environment -->
    <DbConfirmDialog
      :open="envDelete !== null"
      title="Delete environment"
      :description="`Deleting '${envDelete?.name}' permanently removes its secrets and runner tokens.`"
      :confirm-word="envDelete?.name"
      confirm-label="Delete environment"
      :affected-counts="envDeleteCounts"
      :loading="deleteLoading"
      :error="deleteError"
      @confirm="confirmEnvDelete"
      @close="envDelete = null" />
  </aside>
</template>
