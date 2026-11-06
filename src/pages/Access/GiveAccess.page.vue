<script setup lang="ts">
import { computed } from "vue";
import { useGiveAccessController } from "./GiveAccess.controller";
import PrincipalPicker from "./components/PrincipalPicker.vue";
import ProjectPicker from "./components/ProjectPicker.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbButton,
  DbCheckbox,
  DbLevelBadge,
  DbRadioCard,
  DbSearchField,
  DbSkeleton,
} from "~/components/ui";
import { LockIcon } from "~/assets/icons";
import { formatLevel } from "~/utils/format";
import { levelMeter } from "~/utils/level";
import { useAuthStore } from "~/stores/auth.store";

const controller = useGiveAccessController();
const auth = useAuthStore();
/** Without RBAC on the server only AI agents take new access. */
const agentsOnly = computed(() => !auth.hasFeature("rbac"));
const {
  kind,
  forOwnAgent,
  principal,
  principals,
  visiblePrincipals,
  principalFilter,
  chosenKey,
  ready,
  loadError,
  projectFilter,
  manageable,
  visibleProjects,
  projectId,
  project,
  where,
  environmentChoices,
  environmentFilter,
  visibleEnvironments,
  environmentIds,
  level,
  levels,
  planned,
  summary,
  protectedNote,
  parentLabel,
  breadcrumbs,
  saving,
  error,
} = controller;

const fromProject = computed(() => kind.value === "project");

const subtitle = computed(() => {
  if (fromProject.value)
    return agentsOnly.value
      ? `Give an AI agent access to ${parentLabel.value}.`
      : `Give a person, AI agent or group access to ${parentLabel.value}.`;
  const who = principal.value;
  if (!who) return "";
  if (who.type === "group")
    return "Everyone in the group gets this access, but never more than their own role allows.";
  return `Their role allows up to ${formatLevel(who.maxLevel ?? "none")}.`;
});

const allChosen = computed(
  () =>
    environmentChoices.value.length > 0 &&
    environmentIds.value.length === environmentChoices.value.length,
);

function chooseAll(): void {
  environmentIds.value = allChosen.value
    ? []
    : environmentChoices.value.map((choice) => choice.environment.id);
}

const card = "rounded-card border border-line bg-panel p-4 sm:p-5";
const step =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-raised font-mono text-xs text-ink-muted";
const heading = "text-[15px] font-semibold text-ink-strong";
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="!ready" class="flex max-w-3xl flex-col gap-3">
        <DbSkeleton class="h-8 w-72 rounded-control" />
        <DbSkeleton class="h-64 w-full rounded-card" />
      </div>

      <template v-else>
        <PageHeader
          title="Give access"
          :subtitle="subtitle"
          :trail="breadcrumbs" />

        <div
          v-if="manageable.length === 0"
          class="flex max-w-2xl flex-col items-start gap-3">
          <DbAlert tone="info">
            {{
              fromProject
                ? "You don't manage this project, so you can't give access to it. Ask one of its project managers."
                : forOwnAgent
                  ? "You don't reach any project yet, so there's nothing to share with your agent. Ask a project manager for access first."
                  : "You don't manage any project, so you can't give access here. Project managers give access to their own projects."
            }}
          </DbAlert>
          <DbButton size="sm" @click="controller.cancel">Back</DbButton>
        </div>

        <form
          v-else
          class="flex max-w-3xl flex-col gap-4"
          novalidate
          data-testid="give-access-page"
          @submit.prevent="controller.submit">
          <!-- Who, when the project is fixed -->
          <section
            v-if="fromProject"
            :class="card"
            aria-labelledby="who-heading">
            <div class="mb-4 flex items-center gap-3">
              <span :class="step" aria-hidden="true">1</span>
              <h2 id="who-heading" :class="heading">Who</h2>
            </div>
            <PrincipalPicker
              v-if="principals.length > 0"
              v-model:filter="principalFilter"
              :model-value="chosenKey"
              :choices="principals"
              :visible="visiblePrincipals"
              :agents-only="agentsOnly"
              @update:model-value="controller.choosePrincipal" />
            <DbAlert v-else tone="info">
              There's nobody you can list. Ask someone who can see users and
              groups to give access.
            </DbAlert>
            <p
              v-if="principals.length > 0"
              class="mt-2 text-[13px] text-ink-faint">
              <template v-if="agentsOnly">
                AI agents only. People and groups need DOPBASE_RBAC=true on the
                server.
              </template>
              <template v-else>
                People, AI agents and groups. Root already reaches every
                project.
              </template>
            </p>
          </section>

          <!-- Project -->
          <section v-else :class="card" aria-labelledby="project-heading">
            <div class="mb-4 flex items-center gap-3">
              <span :class="step" aria-hidden="true">1</span>
              <h2 id="project-heading" :class="heading">Project</h2>
            </div>
            <ProjectPicker
              v-model:filter="projectFilter"
              :model-value="projectId"
              :choices="manageable"
              :visible="visibleProjects"
              @update:model-value="controller.chooseProject" />
            <p class="mt-2 text-[13px] text-ink-faint">
              {{
                forOwnAgent
                  ? "Your agent can get any project you reach, up to your own level there."
                  : "Only projects you manage are listed."
              }}
            </p>
          </section>

          <!-- Where -->
          <section
            :class="[card, !project && 'opacity-60']"
            aria-labelledby="where-heading">
            <div class="mb-4 flex items-center gap-3">
              <span :class="step" aria-hidden="true">2</span>
              <h2 id="where-heading" :class="heading">Where</h2>
            </div>
            <p v-if="!project" class="text-sm text-ink-muted">
              Choose a project first.
            </p>
            <div v-else class="flex flex-col gap-3">
              <div
                class="grid gap-2 sm:grid-cols-2"
                role="radiogroup"
                aria-labelledby="where-heading">
                <DbRadioCard
                  name="grant-where"
                  value="project"
                  :checked="where === 'project'"
                  label="Whole project"
                  description="Every environment, including new ones."
                  @select="where = 'project'" />
                <DbRadioCard
                  name="grant-where"
                  value="environment"
                  :checked="where === 'environment'"
                  label="Some environments"
                  :description="
                    environmentChoices.length === 0
                      ? 'This project has no environments yet.'
                      : 'Each one gets its own grant.'
                  "
                  :disabled="environmentChoices.length === 0"
                  @select="where = 'environment'" />
              </div>

              <div
                v-if="where === 'environment'"
                class="overflow-hidden rounded-control border border-line"
                data-testid="environment-choices">
                <div
                  class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line-soft bg-raised/50 px-3.5 py-2 text-[13px]">
                  <DbSearchField
                    v-if="environmentChoices.length > 6"
                    v-model="environmentFilter"
                    class="w-full sm:w-56"
                    placeholder="Filter environments" />
                  <span class="text-ink-muted sm:ml-auto">
                    {{ environmentIds.length }} of
                    {{ environmentChoices.length }} chosen
                  </span>
                  <button
                    type="button"
                    class="cursor-pointer text-accent-strong hover:underline"
                    @click="chooseAll">
                    {{ allChosen ? "Clear" : "Choose all" }}
                  </button>
                </div>
                <ul class="max-h-[22rem] overflow-y-auto">
                  <li
                    v-for="choice in visibleEnvironments"
                    :key="choice.environment.id"
                    class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line-soft px-3.5 py-2.5 last:border-b-0">
                    <DbCheckbox
                      :model-value="
                        environmentIds.includes(choice.environment.id)
                      "
                      :label="choice.environment.name"
                      class="font-mono"
                      @update:model-value="
                        controller.toggleEnvironment(
                          choice.environment.id,
                          $event,
                        )
                      " />
                    <span
                      v-if="choice.environment.protected"
                      class="inline-flex items-center gap-1 text-[13px] text-ink-muted">
                      <LockIcon class="h-3.5 w-3.5" />
                      protected
                    </span>
                    <span
                      v-if="choice.current"
                      class="ml-auto flex items-center gap-2 text-[13px] text-ink-muted">
                      has
                      <DbLevelBadge v-bind="levelMeter(choice.current)" />
                    </span>
                  </li>
                </ul>
                <p
                  v-if="visibleEnvironments.length === 0"
                  class="px-3.5 py-4 text-center text-sm text-ink-muted">
                  No environments match "{{ environmentFilter.trim() }}".
                </p>
              </div>
            </div>
          </section>

          <!-- Level -->
          <section :class="card" aria-labelledby="level-heading">
            <div class="mb-4 flex flex-wrap items-center gap-3">
              <span :class="step" aria-hidden="true">3</span>
              <h2 id="level-heading" :class="heading">Level</h2>
              <span
                v-if="principal?.maxLevel"
                class="text-[13px] text-ink-faint sm:ml-auto">
                Their role allows up to {{ formatLevel(principal.maxLevel) }}
              </span>
            </div>
            <div
              class="flex flex-col gap-2"
              role="radiogroup"
              aria-labelledby="level-heading">
              <DbRadioCard
                v-for="choice in levels"
                :key="choice.level"
                name="grant-level"
                :value="choice.level"
                :checked="level === choice.level"
                :label="choice.label"
                :description="choice.hint"
                :disabled="choice.disabled"
                @select="level = choice.level" />
            </div>
          </section>

          <!-- Review -->
          <section
            class="flex flex-col gap-4 rounded-card border bg-panel p-4 sm:p-5"
            :class="planned.length > 0 ? 'border-accent/50' : 'border-line'"
            aria-labelledby="review-heading"
            data-testid="give-access-review">
            <h2 id="review-heading" :class="heading">Review</h2>
            <p
              v-if="planned.length === 0 || !principal"
              class="text-sm text-ink-muted">
              {{
                !principal
                  ? "Choose who gets access to see what changes."
                  : project
                    ? "Choose at least one environment."
                    : "Choose a project to see what changes."
              }}
            </p>
            <template v-else>
              <ul
                class="overflow-hidden rounded-control border border-line text-sm">
                <li
                  v-for="grant in planned"
                  :key="grant.scopeId"
                  class="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 border-b border-line-soft px-3.5 py-2.5 last:border-b-0"
                  data-testid="planned-grant">
                  <span class="min-w-0 truncate font-mono text-ink-strong">
                    {{ grant.label }}
                  </span>
                  <span class="ml-auto flex items-center gap-2">
                    <template v-if="grant.replaces === level">
                      <span class="text-[13px] text-ink-faint">no change</span>
                    </template>
                    <template v-else-if="grant.replaces">
                      <DbLevelBadge v-bind="levelMeter(grant.replaces)" />
                      <span class="text-ink-faint" aria-label="becomes">→</span>
                    </template>
                    <span v-else class="text-[13px] text-ink-faint">new</span>
                    <DbLevelBadge v-bind="levelMeter(level)" />
                  </span>
                </li>
              </ul>
              <DbAlert
                tone="info"
                title="What changes"
                data-testid="give-access-summary">
                {{ summary }}
                <template v-if="protectedNote"> {{ protectedNote }}</template>
              </DbAlert>
            </template>
            <DbAlert v-if="error">{{ error }}</DbAlert>
            <div
              class="flex items-center justify-end gap-2 border-t border-line-soft pt-4">
              <DbButton
                variant="ghost"
                :disabled="saving"
                @click="controller.cancel">
                Cancel
              </DbButton>
              <DbButton
                variant="primary"
                type="submit"
                :loading="saving"
                :disabled="planned.length === 0 || !principal"
                data-testid="give-access-submit">
                Give access
              </DbButton>
            </div>
          </section>
        </form>
      </template>
    </div>
  </DashboardLayout>
</template>
