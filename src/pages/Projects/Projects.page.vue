<script setup lang="ts">
import { ref, watch } from "vue";
import { useProjectsController } from "./Projects.controller";
import ProjectRail from "./components/ProjectRail.vue";
import SecretsPanel from "./components/SecretsPanel.vue";
import TokensPanel from "./components/TokensPanel.vue";
import EnvironmentsOverviewPanel from "./components/EnvironmentsOverviewPanel.vue";
import NameDialog from "./components/NameDialog.vue";
import { DashboardLayout } from "~/layouts";
import {
  DbBadge,
  DbButton,
  DbCopyButton,
  DbEmptyState,
  DbSkeleton,
} from "~/components/ui";
import { BoxIcon, FolderIcon, LayersIcon, SlidersHorizontalIcon } from "~/assets/icons";

/**
 * Projects is the authenticated project and environment screen.
 *
 * Left: project/environment keyline rail. Right: the selected
 * environment's secrets or runner tokens. Empty states explain the
 * project/environment model and offer the primary actions.
 */
const controller = useProjectsController();
const showCreateProject = ref(false);
const projectRailOpen = ref(false);
// Destructure so refs auto-unwrap in the template.
const {
  projects,
  project,
  selectedEnvironment,
  activeTab,
  selectProject,
  isProjectOverview,
  projectRef,
} = controller;

watch(projectRef, () => {
  projectRailOpen.value = false;
});
</script>

<template>
  <DashboardLayout>
    <div class="relative flex min-h-svh flex-col md:flex-row">
      <button
        v-if="projectRailOpen"
        type="button"
        class="fixed inset-0 z-50 bg-ink/40 md:hidden"
        aria-label="Close projects menu"
        @click="projectRailOpen = false" />

      <ProjectRail
        :controller="controller"
        :mobile-open="projectRailOpen"
        @close="projectRailOpen = false" />

      <section class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <header
          class="flex shrink-0 items-center gap-2 border-b border-line-soft px-4 py-3 md:hidden">
          <button
            type="button"
            class="cursor-pointer rounded-control bg-raised p-1.5 text-ink-muted transition-colors hover:bg-line hover:text-ink-strong"
            aria-label="Open projects"
            data-testid="open-project-rail"
            @click="projectRailOpen = true">
            <SlidersHorizontalIcon class="h-4 w-4" />
          </button>
          <span class="min-w-0 truncate font-mono text-sm text-ink-strong">
            {{ project?.name ?? "Projects" }}
          </span>
        </header>
        <!-- No projects yet: explain the model -->
        <div v-if="projects && projects.length === 0" class="p-10">
          <DbEmptyState
            title="No projects yet"
            description="A project is one application or service. Each project holds environments like development, staging, and production, and every environment stores its own encrypted secret values.">
            <template #icon>
              <BoxIcon class="h-5 w-5" />
            </template>
            <template #actions>
              <DbButton variant="primary" @click="showCreateProject = true">
                Create project
              </DbButton>
            </template>
          </DbEmptyState>
        </div>

        <!-- Loading projects skeleton -->
        <div
          v-else-if="!projects"
          class="flex flex-col"
          data-testid="projects-page-skeleton">
          <!-- Header skeleton -->
          <div
            class="flex items-center justify-between border-b border-line px-6 py-4">
            <DbSkeleton class="h-4 w-40" />
            <DbSkeleton class="h-7 w-36 rounded-control" />
          </div>
          <!-- Body skeleton -->
          <div class="p-6">
            <div class="mb-4 flex items-center justify-between gap-4">
              <DbSkeleton class="h-8 w-64 rounded-control" />
              <div class="flex gap-2">
                <DbSkeleton class="h-8 w-24 rounded-control" />
                <DbSkeleton class="h-8 w-24 rounded-control" />
              </div>
            </div>
            <div
              class="overflow-x-auto rounded-card border border-line bg-panel">
              <table class="min-w-full text-left text-sm">
                <thead>
                  <tr class="border-b border-line">
                    <th class="px-4 py-2.5"><DbSkeleton class="h-3 w-16" /></th>
                    <th class="px-4 py-2.5"><DbSkeleton class="h-3 w-16" /></th>
                    <th class="px-4 py-2.5"><DbSkeleton class="h-3 w-20" /></th>
                    <th class="px-4 py-2.5 text-right">
                      <DbSkeleton class="ml-auto h-3 w-16" />
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="i in 5"
                    :key="i"
                    class="border-b border-line-soft last:border-b-0">
                    <td class="px-4 py-3"><DbSkeleton class="h-4 w-36" /></td>
                    <td class="px-4 py-3">
                      <DbSkeleton class="h-4 w-10 rounded-control" />
                    </td>
                    <td class="px-4 py-3"><DbSkeleton class="h-4 w-20" /></td>
                    <td class="px-4 py-3 text-right">
                      <div class="flex items-center justify-end gap-1">
                        <DbSkeleton class="h-6 w-16 rounded-control" />
                        <DbSkeleton class="h-6 w-6 rounded-control" />
                        <DbSkeleton class="h-6 w-6 rounded-control" />
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- All environments overview -->
        <template v-else-if="isProjectOverview && project">
          <header
            class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-b border-line px-6 py-4">
            <nav
              class="flex items-center gap-1.5 font-mono text-sm text-ink-muted">
              <span class="text-ink-strong">{{ project.name }}</span>
              <span class="text-ink-faint">/</span>
              <span class="text-accent-strong">all environments</span>
            </nav>
          </header>
          <div class="flex min-h-0 flex-1 flex-col overflow-hidden p-6">
            <EnvironmentsOverviewPanel
              :controller="controller"
              :project-name="project.name" />
          </div>
        </template>

        <!-- Project selected but has no environments -->
        <div v-else-if="project && !selectedEnvironment" class="p-10">
          <DbEmptyState
            title="No environments"
            description="Environments hold the values a project needs in one context development, staging, production. Create the first one to start storing secrets.">
            <template #icon>
              <LayersIcon class="h-5 w-5" />
            </template>
          </DbEmptyState>
        </div>

        <!-- Selected environment -->
        <template v-else-if="selectedEnvironment">
          <header
            class="flex flex-col gap-3 border-b border-line px-4 py-4 sm:px-6"
            data-testid="environment-header">
            <div
              class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <nav
                class="flex min-w-0 items-center gap-1.5 font-mono text-sm text-ink-muted">
                <button
                  type="button"
                  class="min-w-0 shrink cursor-pointer truncate transition-colors hover:text-ink-strong"
                  @click="selectProject(project!.name)">
                  {{ project!.name }}
                </button>
                <span class="shrink-0 text-ink-faint">/</span>
                <span class="min-w-0 truncate text-ink-strong">
                  {{ selectedEnvironment.name }}
                </span>
              </nav>

              <div
                class="flex w-fit shrink-0 items-center gap-1 self-start rounded-control border border-line bg-panel p-1 sm:self-auto">
              <button
                type="button"
                class="cursor-pointer rounded px-3 py-1 font-mono text-xs transition-colors"
                :class="
                  activeTab === 'secrets'
                    ? 'bg-accent-soft text-ink-strong'
                    : 'text-ink-muted hover:text-ink-strong'
                "
                @click="controller.switchTab('secrets')">
                secrets
              </button>
              <button
                type="button"
                class="cursor-pointer rounded px-3 py-1 font-mono text-xs transition-colors"
                :class="
                  activeTab === 'tokens'
                    ? 'bg-accent-soft text-ink-strong'
                    : 'text-ink-muted hover:text-ink-strong'
                "
                @click="controller.switchTab('tokens')">
                tokens
              </button>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <DbBadge data-testid="environment-id" class="max-w-full truncate">
                {{ selectedEnvironment.id }}
              </DbBadge>
              <DbCopyButton :value="selectedEnvironment.id" label="Copy ID" />
            </div>
          </header>

          <div class="flex min-h-0 flex-1 flex-col overflow-hidden p-6">
            <SecretsPanel
              v-if="activeTab === 'secrets'"
              :environment-id="selectedEnvironment.id"
              :environment-name="selectedEnvironment.name"
              :project-name="project!.name" />
            <TokensPanel v-else :environment-id="selectedEnvironment.id" />
          </div>
        </template>

        <!-- Projects page without a selected project -->
        <div v-else class="p-10">
          <DbEmptyState
            title="Select a project"
            description="Pick a project from the rail, or create one to get started.">
            <template #icon>
              <FolderIcon class="h-5 w-5" />
            </template>
          </DbEmptyState>
        </div>
      </section>
    </div>

    <!-- Create project from the zero state -->
    <NameDialog
      :open="showCreateProject"
      title="New project"
      label="Project name"
      submit-label="Create"
      hint="Names are unique on this server. Example: payment-service"
      :action="controller.createProject"
      @close="showCreateProject = false" />
  </DashboardLayout>
</template>
