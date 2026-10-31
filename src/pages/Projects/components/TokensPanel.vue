<script setup lang="ts">
import { computed, ref } from "vue";
import { useTokensPanelController } from "./TokensPanel.controller";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbEmptyState,
  DbInput,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbModal,
  DbRadioCard,
  DbSearchField,
  DbSegment,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import {
  ActivityIcon,
  CopyIcon,
  InfoIcon,
  KeyIcon,
  PlusIcon,
  XIcon,
} from "~/assets/icons";
import { formatDate } from "~/utils/format";
import { levelMeter } from "~/utils/level";
import type { RunnerToken } from "~/services";
import {
  MAX_TOKEN_EXPIRY_DAYS,
  MAX_TOKEN_EXPIRY_HOURS,
  expiryDate,
  runnerTokenExpiryPresets,
  tokenExpiresIn,
  tokenExpiryUnits,
} from "~/utils/token-expiry";

/**
 * TokensPanel — runner tokens for one environment. A runner token lets a
 * deployed app read this environment's values and nothing else.
 *
 * A new token's plaintext shows once, in a dialog that stays open until
 * it's acknowledged. Revoking takes effect right away.
 */
const props = defineProps<{
  environmentId: string;
  environmentName: string;
  projectName: string;
  protected?: boolean;
}>();

const emit = defineEmits<{
  /** Open this environment's activity. */
  activity: [];
}>();

const environmentIdRef = computed(() => props.environmentId);
const controller = useTokensPanelController(environmentIdRef);
const {
  rows,
  filter,
  copiedId,
  loading,
  loadError,
  actionError,
  creating,
  created,
  acknowledgeCreated,
} = controller;

/** Runner tokens can only read. The meter makes that plain. */
const readMeter = levelMeter("read");
const scope = computed(() => `${props.projectName} / ${props.environmentName}`);

const showCreate = ref(false);
const newName = ref("");
const expiryChoice = ref("90d");
const customAmount = ref("");
const customUnit = ref("d");
const createError = ref<string | null>(null);
const expiryError = ref<string | null>(null);
const revokeTarget = ref<RunnerToken | null>(null);
const revokeLoading = ref(false);
const revokeError = ref<string | null>(null);

function openCreate(): void {
  newName.value = "";
  expiryChoice.value = "90d";
  customAmount.value = "";
  customUnit.value = "d";
  createError.value = null;
  expiryError.value = null;
  showCreate.value = true;
}

/** "deploy-prod can read every value in payments / prod until Jan 25, 2027." */
const createSummary = computed(() => {
  const name = newName.value.trim() || "This token";
  let until: Date | null = null;
  try {
    until = expiryDate(
      tokenExpiresIn(expiryChoice.value, customAmount.value, customUnit.value),
    );
  } catch {
    return `${name} can read every value in ${scope.value}.`;
  }
  return until
    ? `${name} can read every value in ${scope.value} until ${formatDate(until.toISOString())}.`
    : `${name} can read every value in ${scope.value}.`;
});

/** Where the last token was created, for its one-time dialog. */
const createdScope = ref("");

async function submitCreate(): Promise<void> {
  if (newName.value.trim() === "") {
    createError.value = "Enter a token name.";
    return;
  }
  createError.value = null;
  expiryError.value = null;
  let expiresIn: string;
  try {
    expiresIn = tokenExpiresIn(
      expiryChoice.value,
      customAmount.value,
      customUnit.value,
    );
  } catch (error) {
    expiryError.value =
      error instanceof Error ? error.message : "Enter a valid expiry.";
    return;
  }
  createdScope.value = scope.value;
  try {
    await controller.create(newName.value.trim(), expiresIn);
    showCreate.value = false;
  } catch {
    // Keep the dialog open because controller.actionError shows the error.
  }
}

async function confirmRevoke(): Promise<void> {
  if (!revokeTarget.value) return;
  revokeLoading.value = true;
  revokeError.value = null;
  try {
    await controller.revoke(revokeTarget.value);
    revokeTarget.value = null;
  } catch {
    revokeError.value = "The token could not be revoked.";
  } finally {
    revokeLoading.value = false;
  }
}

const createdUsage = computed(() =>
  created.value
    ? [
        `export DOPBASE_TOKEN=${created.value.plaintextToken.slice(0, 12)}…`,
        `dopbase run ${created.value.token.environmentId} -- ./start.sh`,
      ].join("\n")
    : "",
);

const expiresClass: Record<string, string> = {
  default: "text-ink",
  warn: "text-warn",
  crit: "text-ink-faint",
};
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="text-sm leading-[22px] text-ink-muted">
      Runner tokens let a deployed app read one environment's values. For your
      own scripts or an AI agent, use a personal token from your account menu.
    </p>

    <div class="flex flex-wrap items-center gap-2">
      <DbSearchField
        v-model="filter"
        class="w-full sm:w-[232px]"
        placeholder="Filter tokens" />
      <DbButton
        class="ml-auto"
        variant="primary"
        data-testid="tokens-new"
        @click="openCreate">
        <PlusIcon class="h-4 w-4" />
        New runner token
      </DbButton>
    </div>

    <DbAlert v-if="actionError && !showCreate">{{ actionError }}</DbAlert>

    <div
      v-if="loading"
      class="overflow-hidden rounded-card border border-line bg-panel"
      data-testid="tokens-skeleton">
      <div
        v-for="i in 3"
        :key="i"
        class="flex h-11 items-center gap-6 border-b border-line-soft px-4 last:border-b-0">
        <DbSkeleton class="h-4 w-32" />
        <DbSkeleton class="h-4 w-16" />
        <DbSkeleton class="ml-auto h-4 w-24" />
      </div>
    </div>

    <DbAlert v-else-if="loadError">{{ loadError }}</DbAlert>

    <DbEmptyState
      v-else-if="rows && rows.length === 0 && !filter.trim()"
      title="No runner tokens yet"
      description="Create a token for each app or pipeline that reads this environment, so you can revoke one without touching the others.">
      <template #icon>
        <KeyIcon class="h-5 w-5" />
      </template>
    </DbEmptyState>

    <div
      v-else-if="rows"
      class="overflow-x-auto rounded-card border border-line bg-panel">
      <table class="min-w-full text-left text-sm" data-testid="tokens-table">
        <thead>
          <tr class="border-b border-line-soft">
            <th class="h-11 px-4 font-medium text-ink-muted">Name</th>
            <th
              class="hidden h-11 px-4 font-medium text-ink-muted sm:table-cell">
              Can
            </th>
            <th
              class="hidden h-11 px-4 font-medium text-ink-muted md:table-cell">
              Last used
            </th>
            <th class="h-11 px-4 font-medium text-ink-muted">Expires</th>
            <th class="h-11 w-12 px-2"><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="rows.length === 0">
            <td colspan="5" class="px-4 py-8 text-center text-ink-muted">
              No tokens match “{{ filter.trim() }}”.
            </td>
          </tr>
          <tr
            v-for="row in rows"
            :key="row.token.id"
            class="border-b border-line-soft transition-colors last:border-b-0 hover:bg-raised"
            data-testid="token-row">
            <td class="h-11 w-full max-w-0 px-4 sm:w-[34%]">
              <p
                class="truncate font-mono text-sm"
                :class="row.active ? 'text-ink-strong' : 'text-ink-faint'"
                :title="row.token.name">
                {{ row.token.name }}
              </p>
            </td>
            <td class="hidden h-11 px-4 sm:table-cell">
              <DbLevelBadge v-bind="readMeter" />
            </td>
            <td
              class="hidden h-11 whitespace-nowrap px-4 text-ink md:table-cell">
              {{ row.lastUsed }}
            </td>
            <td
              class="h-11 whitespace-nowrap px-4"
              :class="expiresClass[row.expiresTone]">
              {{ copiedId === row.token.id ? "Name copied" : row.expires }}
            </td>
            <td class="h-11 px-2 text-right">
              <DbMenu :label="`Actions for ${row.token.name}`">
                <DbMenuItem @select="controller.copyName(row.token)">
                  <template #icon><CopyIcon class="h-4 w-4" /></template>
                  Copy name
                </DbMenuItem>
                <DbMenuItem @select="emit('activity')">
                  <template #icon><ActivityIcon class="h-4 w-4" /></template>
                  Activity
                </DbMenuItem>
                <template v-if="row.active">
                  <div class="my-0.5 h-px bg-line-soft" role="separator" />
                  <DbMenuItem tone="danger" @select="revokeTarget = row.token">
                    <template #icon><XIcon class="h-4 w-4" /></template>
                    Revoke token
                  </DbMenuItem>
                </template>
              </DbMenu>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create token -->
    <DbModal
      :open="showCreate"
      title="New runner token"
      @close="!creating && (showCreate = false)">
      <form
        id="new-runner-token"
        class="flex flex-col gap-4"
        novalidate
        @submit.prevent="submitCreate">
        <DbInput
          v-model="newName"
          label="Name"
          name="tokenName"
          placeholder="deploy-prod"
          mono
          :error="createError"
          hint="Name it after the app or pipeline that will use it."
          @input="createError = null" />
        <div class="rounded-control bg-raised px-3.5 py-2">
          <p class="text-xs font-semibold text-ink-muted">Environment</p>
          <p class="font-mono text-sm text-ink-strong">
            {{ environmentName
            }}<span v-if="protected" class="font-sans text-ink-muted">
              (protected)</span
            >
          </p>
        </div>
        <div class="flex flex-col gap-1.5">
          <p class="text-sm text-ink-muted">Can</p>
          <DbRadioCard
            name="token-can"
            value="read"
            checked
            label="Read values"
            description="Runner tokens only read. They can't change secrets or see other environments." />
        </div>
        <div class="flex flex-col gap-1.5">
          <p class="text-sm text-ink-muted">Expires</p>
          <DbSegment
            v-model="expiryChoice"
            class="self-start"
            :options="runnerTokenExpiryPresets"
            label="Expires" />
        </div>
        <div v-if="expiryChoice === 'custom'" class="grid grid-cols-2 gap-2">
          <DbInput
            v-model="customAmount"
            label="Duration"
            type="number"
            :min="1"
            :max="
              customUnit === 'h'
                ? MAX_TOKEN_EXPIRY_HOURS
                : MAX_TOKEN_EXPIRY_DAYS
            "
            :step="1"
            :error="expiryError"
            @input="expiryError = null" />
          <DbSelect
            v-model="customUnit"
            label="Unit"
            :options="tokenExpiryUnits" />
        </div>
        <p
          class="flex items-start gap-2.5 rounded-control bg-raised px-3.5 py-3 text-sm text-ink"
          data-testid="token-summary">
          <InfoIcon class="mt-0.5 h-4 w-4 shrink-0 text-accent-strong" />
          {{ createSummary }}
        </p>
        <DbAlert v-if="actionError">{{ actionError }}</DbAlert>
      </form>
      <template #footer>
        <DbButton
          variant="ghost"
          :disabled="creating"
          @click="showCreate = false">
          Cancel
        </DbButton>
        <DbButton
          variant="primary"
          type="submit"
          form="new-runner-token"
          :loading="creating">
          Create token
        </DbButton>
      </template>
    </DbModal>

    <OneTimeTokenDialog
      :id="created?.token.id ?? ''"
      :open="created !== null"
      title="Copy your token"
      :name="created?.token.name ?? ''"
      :token="created?.plaintextToken ?? ''"
      :scope="createdScope"
      :expires="
        created?.token.expiresAt ? formatDate(created.token.expiresAt) : 'Never'
      "
      :usage="createdUsage"
      @acknowledge="acknowledgeCreated" />

    <DbConfirmDialog
      :open="revokeTarget !== null"
      title="Revoke runner token"
      :description="`Apps using '${revokeTarget?.name}' lose access to ${scope} right away.`"
      :confirm-word="revokeTarget?.name"
      confirm-label="Revoke token"
      :loading="revokeLoading"
      :error="revokeError"
      @confirm="confirmRevoke"
      @close="revokeTarget = null" />
  </div>
</template>
