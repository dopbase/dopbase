<script setup lang="ts">
import { computed, ref } from "vue";
import { useProjectController } from "./Project.controller";
import ContextBar from "./components/ContextBar.vue";
import ProjectTabs from "./components/ProjectTabs.vue";
import SecretsPanel from "./components/SecretsPanel.vue";
import ComparePanel from "./components/ComparePanel.vue";
import TokensPanel from "./components/TokensPanel.vue";
import AccessPanel from "./components/AccessPanel.vue";
import ActivityPanel from "./components/ActivityPanel.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import RunDialog from "./components/RunDialog.vue";
import NameDialog from "./components/NameDialog.vue";
import { DashboardLayout } from "~/layouts";
import { DbAlert, DbButton, DbEmptyState, DbSkeleton } from "~/components/ui";
import { LayersIcon, PlusIcon } from "~/assets/icons";
import { useSingleKeyShortcut } from "~/composable";

/**
 * Project — one project, seen from one environment: the context bar
 * (where you are, your level, Run), the project tabs, and the tab's
 * content. Secrets, Compare, Tokens and Activity belong to the
 * environment; Access and Settings to the whole project.
 *
 * A protected environment draws an amber rail across the top. Nothing
 * else in the console uses amber, so production never looks like dev.
 */
const controller = useProjectController();
const {
  projects,
  environments,
  loadError,
  projectsError,
  projectName,
  project,
  environment,
  environmentName,
  environmentMissing,
  activeTab,
  projectLevel,
  tabs,
  meter,
  managesEnvironment,
  managesProject,
} = controller;

const contextBar = ref<InstanceType<typeof ContextBar> | null>(null);
const showCreateEnvironment = ref(false);
const runOpen = ref(false);

/**
 * Whether the open tab may render: it's one of the viewer's tabs and, for
 * project-wide tabs, the project's level is known. Until then nothing
 * mounts, so a tab the viewer can't use never fires its requests.
 */
const tabReady = computed(
  () =>
    tabs.value.some((tab) => tab.key === activeTab.value) &&
    (!projectLevel.value || project.value !== null),
);

const environmentNames = computed(
  () => environments.value?.map((candidate) => candidate.name) ?? [],
);

useSingleKeyShortcut("e", () => contextBar.value?.openEnvironmentMenu());
</script>

<template>
  <DashboardLayout>
    <div
      class="flex min-h-[calc(100svh-3rem)] flex-col lg:min-h-svh"
      :class="environment?.protected ? 'border-t-[3px] border-warn' : ''"
      :data-protected="environment?.protected ? '' : undefined">
      <ContextBar
        ref="contextBar"
        :project-name="projectName"
        :project-id="project?.id ?? null"
        :projects="projects"
        :environments="environments"
        :environment="environment"
        :meter="meter"
        :show-run="environment !== null"
        :can-create-environment="managesProject && project !== null"
        @run="runOpen = true"
        @new-environment="showCreateEnvironment = true"
        @switch-project="controller.switchProject"
        @switch-environment="controller.switchEnvironment" />
      <ProjectTabs v-if="tabs.length > 0" :tabs="tabs" :active="activeTab" />

      <div class="page-width flex-1 py-6">
        <div v-if="loadError" class="flex flex-col items-start gap-3">
          <DbAlert>{{ loadError }}</DbAlert>
          <DbButton size="sm" @click="controller.reload">Try again</DbButton>
        </div>

        <div
          v-else-if="environments === null"
          class="flex flex-col gap-3"
          data-testid="project-skeleton">
          <DbSkeleton class="h-9 w-full max-w-md rounded-control" />
          <DbSkeleton class="h-64 w-full rounded-card" />
        </div>

        <div
          v-else-if="projectLevel && projectsError && !project"
          class="flex flex-col items-start gap-3">
          <DbAlert>{{ projectsError }}</DbAlert>
          <DbButton size="sm" @click="controller.reloadProjects">
            Try again
          </DbButton>
        </div>

        <!-- Access and Settings work without environments; the rest don't. -->
        <DbEmptyState
          v-else-if="
            environments.length === 0 &&
            (!projectLevel || (project !== null && !tabReady))
          "
          class="mx-auto max-w-[520px]"
          title="No environments yet"
          :description="
            managesProject
              ? 'Environments hold the values a project needs in one place: dev, staging or prod. Create the first one to start storing secrets.'
              : 'Nobody has created an environment you can reach in this project yet.'
          ">
          <template #icon>
            <LayersIcon class="h-5 w-5" />
          </template>
          <template v-if="managesProject" #actions>
            <DbButton variant="primary" @click="showCreateEnvironment = true">
              <PlusIcon class="h-4 w-4" />
              New environment
            </DbButton>
          </template>
        </DbEmptyState>

        <DbEmptyState
          v-else-if="environmentMissing"
          class="mx-auto max-w-[520px]"
          :title="`No environment called ${environmentName}`"
          description="It may have been renamed or deleted, or your access to it ended. Pick another environment from the switcher above."
          data-testid="environment-missing">
          <template #icon>
            <LayersIcon class="h-5 w-5" />
          </template>
        </DbEmptyState>

        <div v-else-if="environment && activeTab === 'secrets'">
          <SecretsPanel
            :environment-id="environment.id"
            :environment-name="environment.name"
            :environment-names="environmentNames"
            :project-name="projectName"
            :level="environment.level" />
        </div>

        <ComparePanel
          v-else-if="environment && activeTab === 'compare'"
          :environment="environment"
          :environments="environments"
          :can-create-environment="managesProject && project !== null"
          @new-environment="showCreateEnvironment = true" />

        <TokensPanel
          v-else-if="environment && activeTab === 'tokens' && tabReady"
          :environment-id="environment.id"
          :environment-name="environment.name"
          :project-name="projectName"
          :protected="environment.protected"
          @activity="controller.openActivity" />

        <AccessPanel v-else-if="activeTab === 'access' && tabReady" />

        <ActivityPanel
          v-else-if="activeTab === 'activity'"
          :project-id="project?.id ?? environment?.projectId ?? null"
          :project-name="projectName"
          :environment="environment"
          :environments="environments" />

        <SettingsPanel
          v-else-if="activeTab === 'settings' && tabReady"
          :project="project"
          :environments="environments"
          @changed="controller.refresh" />
      </div>
    </div>

    <RunDialog
      v-if="environment"
      :open="runOpen"
      :project-name="projectName"
      :environment="environment"
      :can-manage="managesEnvironment"
      @close="runOpen = false" />

    <NameDialog
      :open="showCreateEnvironment"
      :title="`New environment in ${projectName}`"
      label="Environment name"
      submit-label="Create"
      hint="Example: development, staging, production"
      placeholder="e.g. production"
      :action="controller.createEnvironment"
      @close="showCreateEnvironment = false" />
  </DashboardLayout>
</template>
