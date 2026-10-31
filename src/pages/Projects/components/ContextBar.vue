<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import {
  CheckIcon,
  ChevronDownIcon,
  FolderIcon,
  LayersIcon,
  LockIcon,
  PlusIcon,
  TerminalIcon,
} from "~/assets/icons";
import {
  DbButton,
  DbKbd,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
} from "~/components/ui";
import { NARROW_SCREEN, useMediaQuery } from "~/composable";
import type { Environment, Project } from "~/services";
import { formatLevel } from "~/utils/format";
import type { LevelMeter } from "~/utils/level";
import SwitcherSheet, { type SwitcherItem } from "./SwitcherSheet.vue";

/**
 * ContextBar — where you are and what you can do there. The breadcrumb
 * segments are the project and environment switchers (E opens the second
 * one), a protected environment gets the amber chip, and your level sits on
 * the right so missing buttons make sense. Run opens how to run an app with
 * this environment's values. On phones the switchers open a bottom sheet
 * instead of a dropdown, and your level and Run sit on their own row.
 * Renders props, reports choices.
 */
const props = defineProps<{
  projectName: string;
  projectId: string | null;
  projects: Project[] | null;
  environments: Environment[] | null;
  environment: Environment | null;
  meter: LevelMeter | null;
  /** Show Run, which opens how to run an app with this environment. */
  showRun: boolean;
  /** Offer New environment in the environment switcher. */
  canCreateEnvironment?: boolean;
}>();

const emit = defineEmits<{
  "switch-project": [project: Project];
  "switch-environment": [environment: Environment];
  run: [];
  "new-environment": [];
}>();

const projectTrigger = ref<HTMLButtonElement | null>(null);
const environmentTrigger = ref<HTMLButtonElement | null>(null);

const narrow = useMediaQuery(NARROW_SCREEN);
const sheet = ref<"project" | "environment" | null>(null);

/** The menu's trigger attributes, or the sheet's on phones. */
function triggerAttrs(
  kind: "project" | "environment",
  menu: Record<string, unknown>,
): Record<string, unknown> {
  if (!narrow.value) return menu;
  return { "aria-haspopup": "dialog", "aria-expanded": sheet.value === kind };
}

function openSwitcher(
  kind: "project" | "environment",
  toggleMenu: () => void,
): void {
  if (narrow.value) sheet.value = kind;
  else toggleMenu();
}

function closeSheet(): void {
  const kind = sheet.value;
  sheet.value = null;
  // Hand focus back to the switcher that opened the sheet.
  void nextTick(() =>
    (kind === "project" ? projectTrigger : environmentTrigger).value?.focus(),
  );
}

const environmentItems = computed<SwitcherItem[] | null>(
  () =>
    props.environments?.map((candidate) => ({
      id: candidate.id,
      name: candidate.name,
      protected: candidate.protected,
      detail: candidate.level ? formatLevel(candidate.level) : undefined,
    })) ?? null,
);

function selectProject(id: string): void {
  const target = props.projects?.find((candidate) => candidate.id === id);
  if (target) emit("switch-project", target);
}

function newEnvironment(): void {
  sheet.value = null;
  emit("new-environment");
}

function selectEnvironment(id: string): void {
  const target = props.environments?.find((candidate) => candidate.id === id);
  if (target) emit("switch-environment", target);
}

/** Opens the environment switcher, for the E shortcut. */
function openEnvironmentMenu(): void {
  environmentTrigger.value?.click();
}

defineExpose({ openEnvironmentMenu });
</script>

<template>
  <div class="border-b border-line" data-testid="context-bar">
    <div
      class="page-width flex min-h-14 flex-wrap items-center gap-x-1.5 gap-y-2 py-2.5">
      <nav aria-label="Breadcrumb" class="flex min-w-0 items-center gap-1.5">
        <DbMenu align="start">
          <template #trigger="{ toggle, attrs }">
            <button
              ref="projectTrigger"
              type="button"
              class="flex min-w-0 cursor-pointer items-center gap-1.5 rounded-control px-1.5 py-1 transition-colors hover:bg-raised pointer-coarse:min-h-11"
              v-bind="triggerAttrs('project', attrs)"
              :aria-label="`Project ${projectName}. Switch project`"
              :title="projectName"
              data-testid="project-switcher"
              @click="openSwitcher('project', toggle)">
              <FolderIcon class="h-4 w-4 shrink-0 text-ink-muted" />
              <!-- Long names truncate; the menu and the title show them whole. -->
              <span
                class="truncate text-[15px] font-semibold leading-5 text-ink-strong sm:max-w-64">
                {{ projectName }}
              </span>
              <ChevronDownIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            </button>
          </template>
          <DbMenuItem
            v-for="candidate in projects ?? []"
            :key="candidate.id"
            :selected="candidate.id === projectId"
            :title="candidate.name"
            @select="emit('switch-project', candidate)">
            <template #icon><FolderIcon class="h-4 w-4" /></template>
            <span class="font-mono">{{ candidate.name }}</span>
            <template v-if="candidate.id === projectId" #trailing>
              <CheckIcon class="h-4 w-4 text-accent-strong" />
            </template>
          </DbMenuItem>
          <p
            v-if="!projects || projects.length === 0"
            class="px-2.5 py-2 text-sm text-ink-muted">
            Loading projects…
          </p>
        </DbMenu>

        <span
          class="shrink-0 text-[15px] leading-6 text-ink-muted"
          aria-hidden="true"
          >/</span
        >

        <DbMenu v-if="environment" align="start">
          <template #trigger="{ toggle, attrs }">
            <button
              ref="environmentTrigger"
              type="button"
              class="flex min-w-0 cursor-pointer items-center gap-1.5 rounded-control border py-1 pl-2 pr-1.5 transition-colors pointer-coarse:min-h-11"
              :class="
                environment.protected
                  ? 'border-warn/45 bg-warn/12 hover:bg-warn/20'
                  : 'border-transparent hover:bg-raised'
              "
              v-bind="triggerAttrs('environment', attrs)"
              :aria-label="`Environment ${environment.name}${environment.protected ? ', protected' : ''}. Switch environment`"
              :title="environment.name"
              data-testid="environment-switcher"
              @click="openSwitcher('environment', toggle)">
              <LockIcon
                v-if="environment.protected"
                class="h-4 w-4 shrink-0 text-warn" />
              <LayersIcon v-else class="h-4 w-4 shrink-0 text-ink-muted" />
              <span
                class="truncate font-mono text-sm font-medium leading-[22px] text-ink-strong sm:max-w-56">
                {{ environment.name }}
              </span>
              <!-- Phones keep the lock and the amber chip, not the word. -->
              <span
                v-if="environment.protected"
                class="hidden shrink-0 text-[13px] leading-[22px] text-warn sm:inline">
                Protected
              </span>
              <ChevronDownIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            </button>
          </template>
          <DbMenuItem
            v-for="candidate in environments ?? []"
            :key="candidate.id"
            :selected="candidate.id === environment.id"
            :title="candidate.name"
            @select="emit('switch-environment', candidate)">
            <template #icon>
              <LockIcon v-if="candidate.protected" class="h-4 w-4 text-warn" />
              <LayersIcon v-else class="h-4 w-4" />
            </template>
            <span class="font-mono">{{ candidate.name }}</span>
            <template #trailing>
              <span
                v-if="candidate.level"
                class="text-[13px] font-normal text-ink-muted">
                {{ formatLevel(candidate.level) }}
              </span>
              <CheckIcon
                v-if="candidate.id === environment.id"
                class="h-4 w-4 text-accent-strong" />
            </template>
          </DbMenuItem>
          <template v-if="canCreateEnvironment">
            <div class="my-0.5 h-px bg-line-soft" role="separator" />
            <DbMenuItem
              data-testid="new-environment"
              @select="emit('new-environment')">
              <template #icon><PlusIcon class="h-4 w-4" /></template>
              New environment
            </DbMenuItem>
          </template>
        </DbMenu>
        <span v-if="environment" class="hidden sm:inline-flex" title="Press E">
          <DbKbd>E</DbKbd>
        </span>
      </nav>

      <!-- Its own left-aligned row on phones; the right end on wider screens. -->
      <div
        class="flex w-full items-center gap-2 sm:ml-auto sm:w-auto"
        data-testid="context-actions">
        <template v-if="meter">
          <span
            class="hidden text-[13px] leading-[22px] text-ink-muted sm:inline">
            Your access
          </span>
          <!-- As tall as Run, so the pair lines up. -->
          <DbLevelBadge v-bind="meter" class="h-10" data-testid="your-access" />
        </template>
        <DbButton
          v-if="showRun"
          class="sm:ml-2"
          data-testid="run-open"
          @click="emit('run')">
          <TerminalIcon class="h-4 w-4" />
          Run
        </DbButton>
      </div>
    </div>

    <SwitcherSheet
      :open="sheet === 'project'"
      title="Switch project"
      kind="project"
      :items="projects"
      :current-id="projectId"
      @select="selectProject"
      @close="closeSheet" />
    <SwitcherSheet
      :open="sheet === 'environment'"
      title="Switch environment"
      kind="environment"
      :items="environmentItems"
      :current-id="environment?.id ?? null"
      :action-label="canCreateEnvironment ? 'New environment' : undefined"
      @select="selectEnvironment"
      @action="newEnvironment"
      @close="closeSheet" />
  </div>
</template>
