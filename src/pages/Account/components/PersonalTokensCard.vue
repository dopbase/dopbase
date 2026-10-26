<script setup lang="ts">
import { computed, ref } from "vue";
import {
  ALL_PROJECTS,
  TOKEN_EXPIRY_OPTIONS,
  TOKEN_LEVEL_OPTIONS,
  usePersonalTokensCardController,
} from "./PersonalTokensCard.controller";
import {
  DbAlert,
  DbButton,
  DbCode,
  DbInput,
  DbModal,
  DbSelect,
} from "~/components/ui";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import { KeyIcon, PlusIcon } from "~/assets/icons";
import { errorMessage } from "~/services/api-errors";
import type { Level } from "~/services/access.api";
import { formatDateTime, formatLevel } from "~/utils/format";

const controller = usePersonalTokensCardController();
const { active, projects, error, created, visible, auditOnly } = controller;

const showCreate = ref(false);
const name = ref("");
const level = ref<Level>("read");
const project = ref(ALL_PROJECTS);
const expiresIn = ref("30d");
const creating = ref(false);
const createError = ref<string | null>(null);

const projectOptions = computed(() => [
  { value: ALL_PROJECTS, label: "Every project I reach" },
  ...projects.value.map((item) => ({ value: item.id, label: item.name })),
]);

function openCreate(): void {
  name.value = "";
  level.value = "read";
  project.value = ALL_PROJECTS;
  expiresIn.value = "30d";
  createError.value = null;
  showCreate.value = true;
}

async function submit(): Promise<void> {
  if (!name.value.trim()) {
    createError.value = "Enter a name, such as the machine or agent using it.";
    return;
  }
  creating.value = true;
  createError.value = null;
  try {
    await controller.create(
      name.value.trim(),
      level.value,
      project.value,
      expiresIn.value,
    );
    showCreate.value = false;
  } catch (cause) {
    createError.value = errorMessage(cause, "Could not create the token.");
  } finally {
    creating.value = false;
  }
}
</script>

<template>
  <section
    v-if="visible"
    class="rounded-card border border-line bg-panel p-5 shadow-xs"
    aria-labelledby="personal-tokens-heading"
    data-testid="personal-tokens-card">
    <div class="flex items-center gap-2">
      <KeyIcon class="h-4 w-4 text-ink-muted" />
      <h2
        id="personal-tokens-heading"
        class="text-sm font-semibold text-ink-strong">
        Personal tokens
      </h2>
      <DbButton
        class="ml-auto"
        size="sm"
        variant="secondary"
        @click="openCreate">
        <PlusIcon class="h-3.5 w-3.5" />
        New token
      </DbButton>
    </div>
    <p class="mt-2 text-xs leading-relaxed text-ink-muted">
      A token acts as you from a script or an AI agent on your machine. It never
      reaches more than you do, and losing access to a project takes it from
      your tokens too. Use it as <DbCode>DOPBASE_TOKEN</DbCode>.
    </p>
    <DbAlert v-if="error" class="mt-3">{{ error }}</DbAlert>
    <ul
      v-if="active.length > 0"
      class="mt-3 overflow-hidden rounded-card border border-line"
      data-testid="personal-tokens">
      <li
        v-for="token in active"
        :key="token.id"
        class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-line-soft px-3 py-2 last:border-0">
        <div class="min-w-0 flex-1">
          <p class="truncate font-mono text-sm text-ink-strong">
            {{ token.name }}
          </p>
          <p class="text-xs text-ink-muted">
            <template v-if="token.scopeType === 'audit'">
              Reads the audit log
            </template>
            <template v-else>
              {{ formatLevel(token.level) }} on
              {{ token.scopeLabel || "every project you reach" }}
            </template>
            · expires
            {{ token.expiresAt ? formatDateTime(token.expiresAt) : "never" }}
          </p>
        </div>
        <DbButton
          size="sm"
          variant="ghost"
          :aria-label="`Revoke ${token.name}`"
          @click="controller.revoke(token)">
          Revoke
        </DbButton>
      </li>
    </ul>
    <p v-else class="mt-3 text-xs text-ink-faint">No active tokens.</p>

    <DbModal
      :open="showCreate"
      title="New personal token"
      @close="showCreate = false">
      <form class="flex flex-col gap-4" @submit.prevent="submit">
        <DbInput v-model="name" label="Name" placeholder="e.g. laptop-agent" />
        <p v-if="auditOnly" class="text-xs text-ink-muted">
          Your tokens read the audit log you can see, for tools such as a SIEM.
          They never reach project data.
        </p>
        <DbSelect
          v-if="!auditOnly"
          :model-value="level"
          label="Can"
          :options="TOKEN_LEVEL_OPTIONS"
          @update:model-value="(value: string) => (level = value as Level)" />
        <DbSelect
          v-if="!auditOnly"
          v-model="project"
          label="Where"
          :options="projectOptions" />
        <DbSelect
          v-model="expiresIn"
          label="Expires"
          :options="TOKEN_EXPIRY_OPTIONS" />
        <p v-if="createError" class="text-xs text-crit">{{ createError }}</p>
        <div class="flex justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="creating"
            @click="showCreate = false">
            Cancel
          </DbButton>
          <DbButton variant="primary" type="submit" :loading="creating">
            Create token
          </DbButton>
        </div>
      </form>
    </DbModal>

    <OneTimeTokenDialog
      :id="created?.token.id ?? ''"
      :open="created !== null"
      title="Your new token"
      :token="created?.plaintextToken ?? ''"
      :name="created?.token.name ?? ''"
      detail="Set it as DOPBASE_TOKEN where your script or agent runs."
      @acknowledge="controller.acknowledge" />
  </section>
</template>
