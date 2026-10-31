<script setup lang="ts">
import type {
  EnvironmentRow,
  ProjectRow,
} from "~/pages/Projects/Projects.controller";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  FolderIcon,
  LayersIcon,
  LockIcon,
  PencilIcon,
  PlusIcon,
  ShieldIcon,
  TrashIcon,
} from "~/assets/icons";
import { DbLevelBadge, DbMenu, DbMenuItem } from "~/components/ui";
import type { Environment, Project } from "~/services";

/**
 * ProjectTree — projects as folders that open like an accordion, with
 * their environments nested inside. Renders the rows the controller built
 * and reports what was clicked. A project row only names the project and
 * counts its environments; access and dates live on environment rows.
 */
defineProps<{
  rows: ProjectRow[];
  /** A filter is applied, so "Show all" doesn't apply. */
  filtering: boolean;
}>();

const emit = defineEmits<{
  toggle: [projectId: string];
  "show-all": [projectId: string];
  open: [environment: Environment];
  "new-environment": [project: Project];
  "rename-project": [project: Project];
  "project-access": [project: Project];
  "delete-project": [project: Project];
  "rename-environment": [environment: Environment];
  "delete-environment": [environment: Environment];
}>();

const environmentWord = (count: number): string =>
  count === 1 ? "1 environment" : `${count} environments`;

const envLabel = (row: EnvironmentRow): string =>
  `${row.environment.projectName} / ${row.environment.name}`;
</script>

<template>
  <div
    class="overflow-hidden rounded-card border border-line"
    data-testid="project-tree">
    <div
      class="hidden h-[34px] items-center gap-4 border-b border-line-soft bg-panel pl-[34px] pr-3 text-sm font-medium leading-5 text-ink-muted sm:flex"
      aria-hidden="true">
      <span class="min-w-0 flex-1">Project / environment</span>
      <span class="w-[190px] shrink-0">Your access</span>
      <span class="hidden w-[110px] shrink-0 md:block">Updated</span>
      <span class="w-8 shrink-0" />
    </div>

    <ul aria-label="Projects">
      <li
        v-for="row in rows"
        :key="row.project.id"
        class="border-t border-line first:border-t-0 sm:first:border-t">
        <div class="flex h-10 items-center gap-4 bg-raised pl-2.5 pr-3">
          <button
            type="button"
            class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left pointer-coarse:min-h-11"
            :aria-expanded="row.expanded"
            @click="emit('toggle', row.project.id)">
            <component
              :is="row.expanded ? ChevronDownIcon : ChevronRightIcon"
              class="h-4 w-4 shrink-0 text-ink-muted" />
            <FolderIcon
              class="h-4 w-4 shrink-0"
              :class="row.expanded ? 'text-accent-strong' : 'text-ink-muted'" />
            <span
              class="truncate font-mono text-sm font-medium leading-[22px] text-ink-strong">
              {{ row.project.name }}
            </span>
            <!-- Read out in full; narrow screens show only the number. -->
            <span
              class="sr-only shrink-0 font-mono text-sm leading-[22px] text-ink-faint sm:not-sr-only">
              {{ environmentWord(row.environmentCount) }}
            </span>
            <span
              class="shrink-0 rounded-control bg-panel px-1.5 font-mono text-sm leading-[22px] text-ink-faint sm:hidden"
              aria-hidden="true">
              {{ row.environmentCount }}
            </span>
          </button>
          <DbMenu
            v-if="row.canManage"
            :label="`Actions for ${row.project.name}`">
            <DbMenuItem @select="emit('new-environment', row.project)">
              <template #icon><PlusIcon class="h-4 w-4" /></template>
              New environment
            </DbMenuItem>
            <DbMenuItem @select="emit('project-access', row.project)">
              <template #icon><ShieldIcon class="h-4 w-4" /></template>
              Access
            </DbMenuItem>
            <DbMenuItem @select="emit('rename-project', row.project)">
              <template #icon><PencilIcon class="h-4 w-4" /></template>
              Rename
            </DbMenuItem>
            <div class="my-0.5 h-px bg-line-soft" role="separator" />
            <DbMenuItem
              tone="danger"
              @select="emit('delete-project', row.project)">
              <template #icon><TrashIcon class="h-4 w-4" /></template>
              Delete project
            </DbMenuItem>
          </DbMenu>
          <span v-else class="w-8 shrink-0" />
        </div>

        <ul
          v-if="row.expanded"
          :aria-label="`Environments in ${row.project.name}`">
          <li
            v-for="env in row.environments"
            :key="env.environment.id"
            class="group flex h-9 items-center gap-4 border-t border-line-soft bg-panel pl-2.5 pr-3 transition-colors hover:bg-raised pointer-coarse:h-11">
            <RouterLink
              :to="{
                name: 'environment',
                params: {
                  projectRef: env.environment.projectName,
                  environmentName: env.environment.name,
                },
              }"
              class="flex h-full min-w-0 flex-1 items-center gap-2"
              :aria-label="`Open secrets in ${envLabel(env)}`"
              @click.prevent="emit('open', env.environment)">
              <span
                class="relative h-full w-[30px] shrink-0"
                aria-hidden="true">
                <span
                  class="absolute inset-y-0 left-[7px] w-px bg-line-strong" />
              </span>
              <LockIcon
                v-if="env.environment.protected"
                class="h-4 w-4 shrink-0 text-warn" />
              <LayersIcon v-else class="h-4 w-4 shrink-0 text-ink-muted" />
              <span
                class="truncate font-mono text-sm leading-[22px] text-ink group-hover:text-ink-strong">
                {{ env.environment.name }}
              </span>
              <span
                v-if="env.environment.protected"
                class="shrink-0 rounded-control bg-warn/15 px-2 py-px font-mono text-sm leading-[22px] text-warn">
                protected
              </span>
              <span
                class="ml-auto hidden shrink-0 items-center gap-1 text-sm font-medium leading-5 text-accent-strong group-hover:flex group-focus-visible:flex">
                Open secrets
                <ChevronRightIcon class="h-3.5 w-3.5" />
              </span>
            </RouterLink>
            <span class="hidden w-[190px] shrink-0 sm:block">
              <DbLevelBadge v-if="env.meter" v-bind="env.meter" />
            </span>
            <span
              class="hidden w-[110px] shrink-0 text-sm leading-[22px] text-ink md:block">
              {{ env.updated }}
            </span>
            <DbMenu
              v-if="env.canManage"
              :label="`Actions for ${envLabel(env)}`">
              <DbMenuItem @select="emit('open', env.environment)">
                <template #icon><LayersIcon class="h-4 w-4" /></template>
                Open secrets
              </DbMenuItem>
              <DbMenuItem @select="emit('rename-environment', env.environment)">
                <template #icon><PencilIcon class="h-4 w-4" /></template>
                Rename
              </DbMenuItem>
              <div class="my-0.5 h-px bg-line-soft" role="separator" />
              <DbMenuItem
                tone="danger"
                @select="emit('delete-environment', env.environment)">
                <template #icon><TrashIcon class="h-4 w-4" /></template>
                Delete environment
              </DbMenuItem>
            </DbMenu>
            <span v-else class="w-8 shrink-0" />
          </li>

          <li
            v-if="row.hiddenCount > 0 && !filtering"
            class="flex h-9 items-center gap-2 border-t border-line-soft bg-panel pl-12 text-sm">
            <button
              type="button"
              class="cursor-pointer font-medium leading-5 text-accent-strong hover:underline"
              @click="emit('show-all', row.project.id)">
              Show all {{ row.environmentCount }} environments
            </button>
            <span class="leading-[22px] text-ink-faint"
              >· or type to filter</span
            >
          </li>

          <li
            v-if="row.environmentCount === 0"
            class="flex h-9 items-center border-t border-line-soft bg-panel pl-12 text-sm leading-[22px] text-ink-muted">
            No environments yet.
          </li>
        </ul>
      </li>
    </ul>
  </div>
</template>
