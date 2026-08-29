<script setup lang="ts">
import { computed, ref } from "vue";
import { useTokensPanelController } from "./TokensPanel.controller";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCode,
  DbConfirmDialog,
  DbCopyButton,
  DbEmptyState,
  DbInput,
  DbModal,
  DbSpinner,
} from "~/components/ui";
import { KeyIcon } from "~/assets/icons";
import { formatRelativeTime, formatDateTime } from "~/utils/format";
import type { RunnerToken } from "~/services";

/**
 * TokensPanel — runner-token management for one environment.
 *
 * A newly created token's plaintext is shown exactly once inside a
 * persistent dialog; closing it after explicit acknowledgment discards the
 * value permanently.
 */
const props = defineProps<{ environmentId: string }>();

const environmentIdRef = computed(() => props.environmentId);
const controller = useTokensPanelController(environmentIdRef);
// Destructure so refs auto-unwrap in the template.
const {
  tokens,
  loading,
  loadError,
  actionError,
  creating,
  created,
  create,
  acknowledgeCreated,
} = controller;

const showCreate = ref(false);
const newName = ref("");
const createError = ref<string | null>(null);
const revokeTarget = ref<RunnerToken | null>(null);
const revokeLoading = ref(false);
const revokeError = ref<string | null>(null);

function openCreate(): void {
  newName.value = "";
  createError.value = null;
  showCreate.value = true;
}

async function submitCreate(): Promise<void> {
  if (newName.value.trim() === "") {
    createError.value = "Enter a token name.";
    return;
  }
  createError.value = null;
  try {
    await create(newName.value.trim());
    showCreate.value = false;
  } catch {
    // Error surfaced via controller.actionError; keep the dialog open.
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

function tokenStatus(token: RunnerToken): {
  label: string;
  tone: "ok" | "crit" | "neutral";
} {
  if (token.revokedAt) return { label: "revoked", tone: "crit" };
  return { label: "active", tone: "ok" };
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <header class="flex flex-wrap items-center gap-2">
      <div>
        <h3 class="text-sm font-semibold">
          Runner tokens
          <DbBadge v-if="tokens" class="ml-1">{{ tokens.length }}</DbBadge>
        </h3>
        <p class="mt-0.5 text-sm text-ink-muted">
          Scoped to this environment only. Run apps with
          <code class="font-mono text-xs">dopbase run --token …</code>
        </p>
      </div>
      <DbButton class="ml-auto" size="sm" variant="primary" @click="openCreate">
        New token
      </DbButton>
    </header>

    <DbAlert v-if="actionError">{{ actionError }}</DbAlert>

    <DbSpinner v-if="loading" class="mx-auto mt-8 h-5 w-5 text-ink-muted" />

    <DbAlert v-else-if="loadError">{{ loadError }}</DbAlert>

    <DbEmptyState
      v-else-if="tokens && tokens.length === 0"
      title="No runner tokens"
      description="Create a token to let one application read this environment's secret values at runtime.">
      <template #icon>
        <KeyIcon class="h-5 w-5" />
      </template>
      <template #actions>
        <DbButton variant="primary" size="sm" @click="openCreate">
          New token
        </DbButton>
      </template>
    </DbEmptyState>

    <div
      v-else-if="tokens"
      class="overflow-x-auto rounded-[var(--radius-card)] border border-line bg-panel">
      <table class="min-w-full text-left text-sm" data-testid="tokens-table">
        <thead>
          <tr class="border-b border-line">
            <th
              class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
              Name
            </th>
            <th
              class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
              Created
            </th>
            <th
              class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
              Last used
            </th>
            <th
              class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
              Status
            </th>
            <th
              class="px-4 py-2.5 text-right text-xs font-medium uppercase tracking-wide text-ink-muted">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="token in tokens"
            :key="token.id"
            class="border-b border-line-soft last:border-b-0">
            <td class="px-4 py-2.5 font-mono text-sm text-ink-strong">
              {{ token.name }}
            </td>
            <td class="px-4 py-2.5 text-sm text-ink-muted">
              {{ formatRelativeTime(token.createdAt) }}
            </td>
            <td class="px-4 py-2.5 text-sm text-ink-muted">
              {{
                token.lastUsedAt
                  ? formatRelativeTime(token.lastUsedAt)
                  : "never"
              }}
            </td>
            <td class="px-4 py-2.5">
              <DbBadge :tone="tokenStatus(token).tone">
                {{ tokenStatus(token).label }}
              </DbBadge>
            </td>
            <td class="px-4 py-2.5 text-right">
              <button
                v-if="!token.revokedAt"
                type="button"
                class="cursor-pointer rounded border border-crit/40 bg-crit/10 px-2 py-1 font-mono text-xs text-crit transition-colors hover:bg-crit/20"
                @click="revokeTarget = token">
                revoke
              </button>
              <span v-else class="text-xs text-ink-muted">
                {{ formatDateTime(token.revokedAt) }}
              </span>
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
        class="flex flex-col gap-4"
        novalidate
        @submit.prevent="submitCreate">
        <DbInput
          v-model="newName"
          label="Token name"
          name="tokenName"
          placeholder="deploy-worker"
          mono
          hint="A readable label, e.g. the deployment target's name." />
        <p class="text-sm text-ink-muted">
          Role: <DbBadge tone="accent">runner</DbBadge>
          — the token can read this environment's runtime values but cannot list
          metadata, mutate secrets, reveal, or export.
        </p>
        <p v-if="createError" class="text-xs text-crit">
          {{ createError }}
        </p>
        <p v-if="actionError" class="text-xs text-crit">
          {{ actionError }}
        </p>
        <div class="flex items-center justify-end gap-2">
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

    <!-- Created token: shown exactly once -->
    <DbModal
      :open="created !== null"
      title="Token created"
      size="md"
      persistent>
      <div v-if="created" class="flex flex-col gap-4">
        <p class="text-sm text-ink">
          Copy the token now. It is stored only as a hash — this plaintext is
          shown <span class="font-semibold text-ink-strong">once</span> and
          cannot be recovered.
        </p>
        <div class="flex flex-wrap items-center gap-3">
          <code
            class="min-w-0 flex-1 break-all rounded border border-accent/30 bg-canvas px-3 py-2 font-mono text-xs text-accent-strong">
            {{ created.plaintextToken }}
          </code>
          <DbCopyButton :value="created.plaintextToken" label="Copy token" />
        </div>
        <DbCode>{{ created.token.name }} · {{ created.token.id }}</DbCode>
        <div class="flex items-center justify-end">
          <DbButton variant="primary" @click="acknowledgeCreated">
            I've stored it safely
          </DbButton>
        </div>
      </div>
    </DbModal>

    <!-- Revoke token -->
    <DbConfirmDialog
      :open="revokeTarget !== null"
      title="Revoke runner token"
      :description="`Applications using '${revokeTarget?.name}' will immediately lose access to this environment.`"
      :confirm-word="revokeTarget?.name"
      confirm-label="Revoke token"
      :loading="revokeLoading"
      :error="revokeError"
      @confirm="confirmRevoke"
      @close="revokeTarget = null" />
  </div>
</template>
