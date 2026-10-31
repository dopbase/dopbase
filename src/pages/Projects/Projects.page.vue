<script setup lang="ts">
import { useProjectsController } from "./Projects.controller";
import ProjectTree from "./components/ProjectTree.vue";
import NameDialog from "./components/NameDialog.vue";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbCopyButton,
  DbEmptyState,
  DbSearchField,
  DbSkeleton,
} from "~/components/ui";
import { FolderIcon, PlusIcon, TerminalIcon } from "~/assets/icons";

/**
 * Projects — every project the viewer reaches, as a tree. Opening a
 * project lists its environments with protection, the viewer's level and
 * the last change; clicking an environment opens its secrets.
 */
const controller = useProjectsController();
const {
  projects,
  loadError,
  filter,
  rows,
  canCreateProjects,
  nameDialog,
  nameDialogCopy,
  deleting,
  deleteCounts,
  deleteLoading,
  deleteError,
} = controller;

const CREATE_COMMAND = "dopbase project create payments";
</script>

<template>
  <DashboardLayout>
    <div class="page-width py-6 lg:py-7">
      <header class="mb-5 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">Projects</h1>
          <p class="mt-1 text-sm text-ink-muted">
            <template v-if="projects && projects.length === 0">
              A project holds the environments and secrets for one app or
              service.
            </template>
            <template v-else>
              Every project you can reach. Open one to see its environments and
              your access in each.
            </template>
          </p>
        </div>
        <DbButton
          v-if="canCreateProjects && projects && projects.length > 0"
          variant="primary"
          data-testid="new-project"
          @click="nameDialog = { kind: 'new-project' }">
          <PlusIcon class="h-4 w-4" />
          New project
        </DbButton>
      </header>

      <div v-if="loadError" class="mb-4 flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" variant="secondary" @click="controller.load">
          Try again
        </DbButton>
      </div>

      <!-- Loading -->
      <div
        v-if="!projects && !loadError"
        class="flex flex-col gap-4"
        data-testid="projects-skeleton">
        <DbSkeleton class="h-9 w-[280px] rounded-control" />
        <div class="overflow-hidden rounded-card border border-line">
          <div
            v-for="i in 5"
            :key="i"
            class="flex h-10 items-center gap-2 border-t border-line bg-raised px-3 first:border-t-0">
            <DbSkeleton class="h-4 w-4 rounded" />
            <DbSkeleton class="h-4 w-40" />
          </div>
        </div>
      </div>

      <!-- No projects yet -->
      <div
        v-else-if="projects && projects.length === 0"
        class="mx-auto mt-10 flex max-w-[520px] flex-col gap-3">
        <DbEmptyState
          :title="
            canCreateProjects
              ? 'Create your first project'
              : 'No projects shared with you'
          "
          :description="
            canCreateProjects
              ? 'Add a project, then its environments (dev, staging, prod) and secrets. You can import an existing .env file.'
              : 'Ask a project manager to give you access to a project.'
          ">
          <template #icon>
            <FolderIcon class="h-5 w-5" />
          </template>
          <template v-if="canCreateProjects" #actions>
            <DbButton
              variant="primary"
              data-testid="new-project"
              @click="nameDialog = { kind: 'new-project' }">
              <PlusIcon class="h-4 w-4" />
              New project
            </DbButton>
          </template>
        </DbEmptyState>
        <div
          v-if="canCreateProjects"
          class="rounded-card border border-line bg-panel">
          <div
            class="flex items-center gap-2 border-b border-line-soft px-3 py-2 text-sm text-ink-muted">
            <TerminalIcon class="h-4 w-4" />
            Or from your terminal
            <DbCopyButton
              class="ml-auto"
              :value="CREATE_COMMAND"
              label="Copy" />
          </div>
          <pre
            class="overflow-x-auto px-3 py-3 font-mono text-sm text-ink-strong"><code>{{ CREATE_COMMAND }}</code></pre>
        </div>
      </div>

      <!-- The tree -->
      <template v-else-if="projects">
        <DbSearchField
          v-model="filter"
          class="mb-4 w-full sm:w-[280px]"
          placeholder="Filter projects and environments" />
        <ProjectTree
          v-if="rows.length > 0"
          :rows="rows"
          :filtering="filter.trim() !== ''"
          @toggle="controller.toggleProject"
          @show-all="controller.showAllEnvironments"
          @open="controller.openEnvironment"
          @project-access="controller.openProjectAccess"
          @new-environment="
            (project) => (nameDialog = { kind: 'new-environment', project })
          "
          @rename-project="
            (project) => (nameDialog = { kind: 'rename-project', project })
          "
          @delete-project="
            (project) => controller.startDelete({ kind: 'project', project })
          "
          @rename-environment="
            (environment) =>
              (nameDialog = { kind: 'rename-environment', environment })
          "
          @delete-environment="
            (environment) =>
              controller.startDelete({ kind: 'environment', environment })
          " />
        <p
          v-else
          class="rounded-card border border-dashed border-line px-4 py-8 text-center text-sm text-ink-muted"
          data-testid="projects-no-match">
          Nothing matches “{{ filter.trim() }}”.
        </p>
      </template>
    </div>

    <NameDialog
      :open="nameDialog !== null"
      :title="nameDialogCopy.title"
      :label="nameDialogCopy.label"
      :initial-name="nameDialogCopy.initialName"
      :submit-label="nameDialogCopy.submitLabel"
      :hint="nameDialogCopy.hint"
      :placeholder="nameDialogCopy.placeholder"
      :action="controller.submitName"
      @close="nameDialog = null" />

    <DbConfirmDialog
      :open="deleting !== null"
      :title="
        deleting?.kind === 'environment'
          ? 'Delete environment'
          : 'Delete project'
      "
      :description="
        deleting?.kind === 'environment'
          ? `Deleting '${deleting.environment.name}' permanently removes its secrets and runner tokens.`
          : `Deleting '${deleting?.project.name}' permanently removes it with all environments, secrets, and scoped runner tokens.`
      "
      :confirm-word="
        deleting?.kind === 'environment'
          ? deleting.environment.name
          : deleting?.project.name
      "
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
  </DashboardLayout>
</template>
