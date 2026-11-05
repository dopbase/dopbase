<script setup lang="ts">
import {
  LIFETIME_OPTIONS,
  MAX_LEVEL_OPTIONS,
  TOKEN_LEVEL_OPTIONS,
  useRoleController,
} from "./Role.controller";
import { CAPABILITY_GROUPS, CAPABILITY_LABELS } from "./Roles.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCheckbox,
  DbInput,
  DbLevelBadge,
  DbRadioCard,
  DbSegment,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import { CopyIcon, InfoIcon } from "~/assets/icons";
import type { Level } from "~/services/access.api";
import type { SignIn } from "~/services/roles.api";
import { formatLevel } from "~/utils/format";
import { LEVEL_HINTS, levelMeter } from "~/utils/level";

const controller = useRoleController();
const {
  roles,
  role,
  isNew,
  readOnly,
  loadError,
  notFound,
  title,
  name,
  description,
  maxLevel,
  signIn,
  tokenScope,
  tokenMaxLevel,
  tokenLifetime,
  holdsTokens,
  summary,
  saving,
  nameError,
  saveError,
} = controller;

const SIGN_IN_CHOICES: {
  value: SignIn;
  label: string;
  hint: string;
  disabled: boolean;
}[] = [
  {
    value: "interactive",
    label: "Password or single sign-on",
    hint: "For people who use the console.",
    disabled: false,
  },
  {
    value: "token_only",
    label: "Token only",
    hint: "For AI agents and CI. Each one needs an owner.",
    disabled: true,
  },
];
const TOKEN_SCOPE_OPTIONS = [
  { value: "data", label: "Project data" },
  { value: "audit", label: "The audit log only" },
];

/** A section: its explanation beside its fields from lg, above them below. */
const card =
  "grid gap-4 rounded-card border border-line bg-panel p-4 sm:p-6 lg:grid-cols-[300px_minmax(0,1fr)] lg:gap-8";
const heading = "text-[15px] font-semibold text-ink-strong";
const explain = "mt-1 text-[13px] leading-5 text-ink-muted";
/** A Summary fact: label on the left, value and note on the right. */
const fact =
  "flex items-start justify-between gap-4 border-t border-line-soft py-3";
const factBody = "flex min-w-0 flex-col items-end gap-1 text-right";
const factLabel =
  "shrink-0 pt-0.5 font-mono text-[11px] uppercase tracking-wide text-ink-faint";
const factValue = "text-sm font-semibold text-ink-strong";
const factNote = "hidden text-xs leading-5 text-ink-muted sm:block";
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="notFound" class="flex flex-col items-start gap-3">
        <DbAlert>That role doesn't exist any more.</DbAlert>
        <DbButton size="sm" @click="$router.push({ name: 'roles' })">
          Back to roles
        </DbButton>
      </div>

      <div v-else-if="roles === null" class="flex flex-col gap-3">
        <DbSkeleton class="h-8 w-72 rounded-control" />
        <DbSkeleton class="h-64 w-full rounded-card" />
      </div>

      <form
        v-else
        class="flex flex-col gap-4"
        novalidate
        data-testid="role-editor"
        @submit.prevent="controller.save">
        <PageHeader
          :title="title"
          :subtitle="
            role
              ? `${role.members} ${role.members === 1 ? 'account holds' : 'accounts hold'} this role.`
              : 'Nobody holds it until you give it to someone.'
          "
          :parent="{ label: 'Roles', to: { name: 'roles' } }">
          <template v-if="readOnly" #actions>
            <DbButton
              data-testid="duplicate-role"
              @click="controller.duplicate">
              <CopyIcon class="h-4 w-4" />
              Duplicate
            </DbButton>
          </template>
        </PageHeader>

        <DbAlert v-if="readOnly" tone="info">
          Built-in roles can't be changed. Duplicate this one to start a custom
          role from its settings.
        </DbAlert>
        <DbAlert v-else-if="!isNew && role?.members" tone="info">
          Changes apply to all {{ role.members }}
          {{ role.members === 1 ? "holder" : "holders" }} on their next request.
        </DbAlert>

        <!-- Basics -->
        <section :class="card" aria-labelledby="basics-heading">
          <div>
            <h2 id="basics-heading" :class="heading">Basics</h2>
            <p :class="explain">
              Shown on Users, in role pickers and in the audit log.
            </p>
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <DbInput
              v-model="name"
              label="Name"
              name="roleName"
              placeholder="Viewer"
              :readonly="readOnly"
              :error="nameError" />
            <DbInput
              v-model="description"
              label="Description (optional)"
              name="roleDescription"
              placeholder="Reads values, never changes them"
              :readonly="readOnly" />
          </div>
        </section>

        <!-- Project data -->
        <section :class="card" aria-labelledby="data-heading">
          <div>
            <h2 id="data-heading" :class="heading">Project data</h2>
            <p :class="explain">
              The most any grant can give holders. A grant above it is cut down
              to it. Protected environments still need a grant of their own.
            </p>
          </div>
          <div class="flex min-w-0 flex-col gap-2">
            <DbLevelBadge
              v-if="readOnly"
              v-bind="levelMeter(maxLevel)"
              class="self-start" />
            <template v-else>
              <!-- Five levels don't fit on a phone, so it's a select there. -->
              <div
                class="hidden max-w-full self-start overflow-x-auto sm:block">
                <DbSegment
                  :model-value="maxLevel"
                  :options="MAX_LEVEL_OPTIONS"
                  label="Reaches at most"
                  @update:model-value="maxLevel = $event as Level" />
              </div>
              <DbSelect
                :model-value="maxLevel"
                :options="MAX_LEVEL_OPTIONS"
                label="Reaches at most"
                class="sm:hidden"
                @update:model-value="maxLevel = $event as Level" />
            </template>
            <p class="text-sm text-ink-muted">{{ LEVEL_HINTS[maxLevel] }}</p>
          </div>
        </section>

        <!-- Capabilities -->
        <section :class="card" aria-labelledby="caps-heading">
          <div>
            <h2 id="caps-heading" :class="heading">What holders can do</h2>
            <p :class="explain">
              Powers outside project data. Root-only powers aren't listed.
              <span data-testid="capability-count">
                {{ summary.held }} of {{ summary.total }} picked.
              </span>
            </p>
          </div>
          <div class="grid gap-x-6 gap-y-5 sm:grid-cols-2">
            <fieldset
              v-for="group in CAPABILITY_GROUPS"
              :key="group.label"
              class="flex min-w-0 flex-col gap-2.5">
              <legend
                class="mb-2 font-mono text-xs uppercase tracking-wide text-ink-faint">
                {{ group.label }}
              </legend>
              <div
                v-for="capability in group.capabilities"
                :key="capability"
                class="flex flex-col gap-0.5"
                data-testid="capability">
                <DbCheckbox
                  :model-value="controller.has(capability)"
                  :label="CAPABILITY_LABELS[capability]"
                  :disabled="readOnly"
                  @update:model-value="controller.toggle(capability, $event)" />
                <span class="pl-[30px] font-mono text-xs text-ink-faint">
                  {{ capability }}
                </span>
              </div>
            </fieldset>
          </div>
        </section>

        <!-- Personal tokens -->
        <section
          v-if="holdsTokens"
          :class="card"
          aria-labelledby="tokens-heading"
          data-testid="token-policy">
          <div>
            <h2 id="tokens-heading" :class="heading">Personal tokens</h2>
            <p :class="explain">
              What tokens created by holders can do. A token never reaches more
              than its owner.
            </p>
          </div>
          <div
            v-if="readOnly"
            class="grid grid-cols-[10rem_1fr] gap-y-2 text-sm">
            <span class="text-ink-muted">Tokens reach</span>
            <span class="text-ink">
              {{
                tokenScope === "audit" ? "The audit log only" : "Project data"
              }}
            </span>
            <template v-if="tokenScope === 'data'">
              <span class="text-ink-muted">Tokens can</span>
              <span class="text-ink">{{ formatLevel(tokenMaxLevel) }}</span>
            </template>
            <span class="text-ink-muted">Longest lifetime</span>
            <span class="text-ink">{{ tokenLifetime }} days</span>
          </div>
          <div
            v-else
            class="flex flex-col gap-4 lg:flex-row lg:items-end lg:gap-6">
            <DbSelect
              :model-value="tokenScope"
              label="Tokens reach"
              :options="TOKEN_SCOPE_OPTIONS"
              class="lg:min-w-0 lg:flex-1"
              @update:model-value="tokenScope = $event as 'data' | 'audit'" />
            <div v-if="tokenScope === 'data'" class="flex flex-col gap-1.5">
              <p class="text-sm font-medium text-ink-muted">Tokens can</p>
              <DbSegment
                :model-value="tokenMaxLevel"
                :options="TOKEN_LEVEL_OPTIONS"
                label="Tokens can"
                class="self-start"
                @update:model-value="tokenMaxLevel = $event as Level" />
            </div>
            <div class="flex flex-col gap-1.5">
              <p class="text-sm font-medium text-ink-muted">Longest lifetime</p>
              <DbSegment
                v-model="tokenLifetime"
                :options="LIFETIME_OPTIONS"
                label="Longest lifetime"
                class="self-start" />
            </div>
          </div>
        </section>

        <!-- Hidden on New role for now: picking a sign-in method comes in a
             later release. Existing roles still show theirs. -->
        <section v-if="!isNew" :class="card" aria-labelledby="signin-heading">
          <div>
            <h2 id="signin-heading" :class="heading">Signs in with</h2>
            <p :class="[explain, 'flex items-start gap-1.5']">
              <InfoIcon class="mt-0.5 h-4 w-4 shrink-0" />
              Picked once, when the role is created.
            </p>
          </div>
          <div>
            <div
              v-if="isNew"
              class="flex flex-col gap-2"
              role="radiogroup"
              aria-label="Signs in with">
              <DbRadioCard
                v-for="choice in SIGN_IN_CHOICES"
                :key="choice.value"
                name="role-sign-in"
                :value="choice.value"
                :checked="signIn === choice.value"
                :label="choice.label"
                :description="choice.hint"
                :disabled="choice.disabled"
                @select="signIn = $event as SignIn" />
            </div>
            <p class="text-sm text-ink">
              {{
                SIGN_IN_CHOICES.find((choice) => choice.value === signIn)?.label
              }}
            </p>
          </div>
        </section>

        <!-- Summary: what the role adds up to, right before saving -->
        <section
          class="rounded-card border border-line bg-panel p-4 sm:p-6"
          aria-labelledby="summary-heading"
          data-testid="role-summary">
          <h2 id="summary-heading" :class="heading">Summary</h2>
          <p :class="explain">
            {{
              readOnly
                ? "What holders of this role get."
                : isNew
                  ? "Check this before you create the role. It updates as you edit."
                  : "Check this before you save. It updates as you edit."
            }}
          </p>
          <div
            v-if="summary.warnings.length > 0"
            class="mt-4 flex flex-col gap-2">
            <DbAlert
              v-for="warning in summary.warnings"
              :key="warning.title"
              tone="warn"
              :title="warning.title">
              {{ warning.body }}
            </DbAlert>
          </div>
          <dl class="mt-4 grid lg:grid-cols-2 lg:gap-x-10">
            <div :class="fact">
              <dt :class="factLabel">Reaches at most</dt>
              <dd :class="factBody">
                <DbLevelBadge v-bind="levelMeter(summary.maxLevel)" />
                <span :class="factNote">Higher grants are cut down</span>
              </dd>
            </div>
            <div :class="fact">
              <dt :class="factLabel">Holders can</dt>
              <dd :class="factBody">
                <span :class="factValue">
                  {{ summary.held }} of {{ summary.total }}
                </span>
                <span :class="factNote">{{ summary.heldList }}</span>
              </dd>
            </div>
            <div :class="fact">
              <dt :class="factLabel">Their tokens</dt>
              <dd :class="factBody">
                <span :class="factValue">{{ summary.tokens }}</span>
                <span :class="factNote">{{ summary.tokensDetail }}</span>
              </dd>
            </div>
            <div :class="fact">
              <dt :class="factLabel">Who can give it</dt>
              <dd :class="factBody">
                <span :class="factValue">{{ summary.givers }}</span>
                <span :class="factNote">{{ summary.giversDetail }}</span>
              </dd>
            </div>
            <div v-if="summary.closest" :class="fact">
              <dt :class="factLabel">Closest built-in</dt>
              <dd :class="factBody">
                <DbBadge>{{ summary.closest.name }}</DbBadge>
                <span :class="factNote">{{ summary.closestDetail }}</span>
              </dd>
            </div>
          </dl>
        </section>

        <DbAlert v-if="saveError">{{ saveError }}</DbAlert>

        <!-- Phones: full width, the primary first. Wider: right-aligned. -->
        <div
          v-if="!readOnly"
          class="flex flex-col gap-2 sm:flex-row-reverse sm:justify-start">
          <DbButton
            variant="primary"
            type="submit"
            class="w-full sm:w-auto"
            :loading="saving"
            data-testid="save-role">
            {{ isNew ? "Create role" : "Save role" }}
          </DbButton>
          <DbButton
            variant="secondary"
            class="w-full sm:w-auto"
            :disabled="saving"
            @click="$router.push({ name: 'roles' })">
            Cancel
          </DbButton>
        </div>
      </form>
    </div>
  </DashboardLayout>
</template>
