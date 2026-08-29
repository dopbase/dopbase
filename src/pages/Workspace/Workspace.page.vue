<script setup lang="ts">
import { ref } from "vue";
import { useWorkspaceController } from "./Workspace.controller";
import ProjectRail from "./components/ProjectRail.vue";
import SecretsPanel from "./components/SecretsPanel.vue";
import TokensPanel from "./components/TokensPanel.vue";
import NameDialog from "./components/NameDialog.vue";
import { DashboardLayout } from "~/layouts";
import { DbButton, DbEmptyState, DbSpinner } from "~/components/ui";
import { BoxIcon, FolderIcon, LayersIcon } from "~/assets/icons";

/**
 * Workspace — the authenticated landing workspace.
 *
 * Left: project/environment keyline rail. Right: the selected
 * environment's secrets or runner tokens. Empty states explain the
 * project/environment model and offer the primary actions.
 */
const controller = useWorkspaceController();
const showCreateProject = ref(false);
// Destructure so refs auto-unwrap in the template.
const { projects, project, selectedEnvironment, activeTab, selectProject } =
  controller;
</script>

<template>
  <DashboardLayout>
    <div class="flex min-h-svh">
      <ProjectRail :controller="controller" />

      <section class="min-w-0 flex-1">
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

        <!-- Loading projects -->
        <div
          v-else-if="!projects"
          class="flex h-full items-center justify-center p-10">
          <DbSpinner class="h-6 w-6 text-ink-muted" />
        </div>

        <!-- Project selected but has no environments -->
        <div v-else-if="project && !selectedEnvironment" class="p-10">
          <DbEmptyState
            title="No environments"
            description="Environments hold the values a project needs in one context — development, staging, production. Create the first one to start storing secrets.">
            <template #icon>
              <LayersIcon class="h-5 w-5" />
            </template>
          </DbEmptyState>
        </div>

        <!-- Selected environment -->
        <template v-else-if="selectedEnvironment">
          <header
            class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line px-6 py-4">
            <nav
              class="flex items-center gap-1.5 font-mono text-sm text-ink-muted">
              <button
                type="button"
                class="cursor-pointer transition-colors hover:text-ink-strong"
                @click="selectProject(project!.name)">
                {{ project!.name }}
              </button>
              <span class="text-ink-faint">/</span>
              <span class="text-ink-strong">
                {{ selectedEnvironment.name }}
              </span>
            </nav>

            <div
              class="ml-auto flex items-center gap-1 rounded-md border border-line bg-panel p-1">
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
          </header>

          <div class="p-6">
            <SecretsPanel
              v-if="activeTab === 'secrets'"
              :environment-id="selectedEnvironment.id"
              :environment-name="selectedEnvironment.name"
              :project-name="project!.name" />
            <TokensPanel v-else :environment-id="selectedEnvironment.id" />
          </div>
        </template>

        <!-- Workspace without project param (e.g. zero-state navigation) -->
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
