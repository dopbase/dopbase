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
  saving,
  nameError,
  saveError,
} = controller;

const SIGN_IN_CHOICES: { value: SignIn; label: string; hint: string }[] = [
  {
    value: "interactive",
    label: "Password or single sign-on",
    hint: "For people who use the console.",
  },
  {
    value: "token_only",
    label: "Token only",
    hint: "For AI agents and CI. Each one needs an owner.",
  },
];
const TOKEN_SCOPE_OPTIONS = [
  { value: "data", label: "Project data" },
  { value: "audit", label: "The audit log only" },
];

const card = "rounded-card border border-line bg-panel p-4 sm:p-5";
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
        class="flex max-w-3xl flex-col gap-5"
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
          <h2
            id="basics-heading"
            class="mb-4 text-[15px] font-semibold text-ink-strong">
            Basics
          </h2>
          <div class="flex flex-col gap-4">
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
          <h2
            id="data-heading"
            class="text-[15px] font-semibold text-ink-strong">
            Project data
          </h2>
          <p class="mb-4 mt-1 text-sm text-ink-muted">
            The most any grant can give holders of this role. Higher grants are
            cut down to this level.
          </p>
          <DbLevelBadge v-if="readOnly" v-bind="levelMeter(maxLevel)" />
          <template v-else>
            <DbSegment
              :model-value="maxLevel"
              :options="MAX_LEVEL_OPTIONS"
              label="Reaches at most"
              class="max-w-full overflow-x-auto"
              @update:model-value="maxLevel = $event as Level" />
          </template>
          <p class="mt-2 text-sm text-ink-muted">{{ LEVEL_HINTS[maxLevel] }}</p>
        </section>

        <!-- Capabilities -->
        <section :class="card" aria-labelledby="caps-heading">
          <h2
            id="caps-heading"
            class="text-[15px] font-semibold text-ink-strong">
            What holders can do on this instance
          </h2>
          <p class="mb-4 mt-1 text-sm text-ink-muted">
            Each capability is a power outside project data. Root-only powers
            aren't listed.
          </p>
          <div class="grid gap-5 sm:grid-cols-2">
            <fieldset
              v-for="group in CAPABILITY_GROUPS"
              :key="group.label"
              class="flex flex-col gap-2.5">
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
          <h2
            id="tokens-heading"
            class="text-[15px] font-semibold text-ink-strong">
            Personal tokens
          </h2>
          <p class="mb-4 mt-1 text-sm text-ink-muted">
            What tokens created by holders can do. A token never reaches more
            than its owner.
          </p>
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
          <div v-else class="flex flex-col gap-4">
            <DbSelect
              :model-value="tokenScope"
              label="Tokens reach"
              :options="TOKEN_SCOPE_OPTIONS"
              class="max-w-xs"
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

        <!-- Sign-in -->
        <section :class="card" aria-labelledby="signin-heading">
          <h2
            id="signin-heading"
            class="text-[15px] font-semibold text-ink-strong">
            Signs in with
          </h2>
          <p class="mb-4 mt-1 flex items-start gap-1.5 text-sm text-ink-muted">
            <InfoIcon class="mt-0.5 h-4 w-4 shrink-0" />
            Picked once, when the role is created.
          </p>
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
              @select="signIn = $event as SignIn" />
          </div>
          <p v-else class="text-sm text-ink">
            {{
              SIGN_IN_CHOICES.find((choice) => choice.value === signIn)?.label
            }}
          </p>
        </section>

        <DbAlert v-if="saveError">{{ saveError }}</DbAlert>

        <div v-if="!readOnly" class="flex justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="saving"
            @click="$router.push({ name: 'roles' })">
            Cancel
          </DbButton>
          <DbButton
            variant="primary"
            type="submit"
            :loading="saving"
            data-testid="save-role">
            {{ isNew ? "Create role" : "Save role" }}
          </DbButton>
        </div>
      </form>
    </div>
  </DashboardLayout>
</template>
