<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useWorkspaceTokensPanelController } from "./WorkspaceTokensPanel.controller";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbEmptyState,
  DbInput,
  DbModal,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import { KeyIcon } from "~/assets/icons";
import { formatRelativeTime, formatDateTime } from "~/utils/format";
import type { WorkspaceToken } from "~/services";
import {
  MAX_TOKEN_EXPIRY_DAYS,
  MAX_TOKEN_EXPIRY_HOURS,
  tokenExpiresIn,
  tokenExpiryOptions,
  tokenExpiryUnits,
} from "~/utils/token-expiry";

const props = defineProps<{ workspaceId: string }>();

const workspaceIdRef = computed(() => props.workspaceId);
const controller = useWorkspaceTokensPanelController(workspaceIdRef);
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
const expiryChoice = ref("never");
const customAmount = ref("");
const customUnit = ref("h");
const createError = ref<string | null>(null);
const expiryError = ref<string | null>(null);
const revokeTarget = ref<WorkspaceToken | null>(null);
const revokeLoading = ref(false);
const revokeError = ref<string | null>(null);
const now = ref(Date.now());
let expiryTimer: ReturnType<typeof setTimeout> | undefined;

function scheduleExpiry(): void {
  clearTimeout(expiryTimer);
  now.value = Date.now();
  const futureExpiries = tokens.value
    ?.filter((token) => !token.revokedAt && token.expiresAt)
    .map((token) => new Date(token.expiresAt!).getTime())
    .filter((expiry) => expiry > now.value);
  if (!futureExpiries?.length) return;
  const nextExpiry = Math.min(...futureExpiries);
  expiryTimer = setTimeout(
    scheduleExpiry,
    Math.min(nextExpiry - now.value + 1, 2_147_483_647),
  );
}

watch(tokens, scheduleExpiry, { immediate: true });
onUnmounted(() => clearTimeout(expiryTimer));

function openCreate(): void {
  newName.value = "";
  expiryChoice.value = "never";
  customAmount.value = "";
  customUnit.value = "h";
  createError.value = null;
  expiryError.value = null;
  showCreate.value = true;
}

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
  try {
    await create(newName.value.trim(), expiresIn);
    showCreate.value = false;
  } catch {
    /* actionError */
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

function tokenStatus(token: WorkspaceToken): {
  label: string;
  tone: "ok" | "crit" | "neutral";
} {
  if (token.revokedAt) return { label: "revoked", tone: "crit" };
  if (token.expiresAt && new Date(token.expiresAt).getTime() <= now.value)
    return { label: "expired", tone: "crit" };
  return { label: "active", tone: "ok" };
}
</script>

<template>
  <div class="flex max-h-[min(70vh,32rem)] flex-col gap-4 overflow-y-auto">
    <header
      class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div class="min-w-0">
        <h3 class="text-sm font-semibold">
          Workspace tokens
          <DbBadge v-if="tokens" class="ml-1">{{ tokens.length }}</DbBadge>
        </h3>
        <p class="mt-0.5 text-sm text-ink-muted">
          Runtime access to every linked project and environment in this
          workspace.
        </p>
      </div>
      <DbButton
        class="w-fit shrink-0 self-start sm:self-auto"
        size="sm"
        variant="primary"
        @click="openCreate">
        New token
      </DbButton>
    </header>

    <DbAlert v-if="actionError">{{ actionError }}</DbAlert>

    <DbAlert v-if="loadError">{{ loadError }}</DbAlert>

    <DbEmptyState
      v-else-if="!loading && tokens && tokens.length === 0"
      title="No workspace tokens"
      description="Create a token for CI or hosts that need secrets across multiple projects in this workspace.">
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
      class="overflow-x-auto rounded-card border border-line bg-panel">
      <table class="min-w-full text-left text-sm" data-testid="workspace-tokens-table">
        <thead>
          <tr class="border-b border-line">
            <th class="px-4 py-2.5 text-xs font-medium uppercase text-ink-muted">
              Name
            </th>
            <th class="px-4 py-2.5 text-xs font-medium uppercase text-ink-muted">
              Expires
            </th>
            <th class="px-4 py-2.5 text-xs font-medium uppercase text-ink-muted">
              Status
            </th>
            <th class="px-4 py-2.5 text-right text-xs font-medium uppercase text-ink-muted">
              Actions
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="token in tokens"
            :key="token.id"
            class="border-b border-line-soft last:border-b-0">
            <td class="px-4 py-2.5 font-mono text-sm">{{ token.name }}</td>
            <td class="px-4 py-2.5 text-sm text-ink-muted">
              {{ token.expiresAt ? formatDateTime(token.expiresAt) : "never" }}
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
                class="cursor-pointer rounded border border-crit/40 bg-crit/10 px-2 py-1 font-mono text-xs text-crit"
                @click="revokeTarget = token">
                revoke
              </button>
              <span v-else class="text-xs text-ink-muted">
                {{ formatRelativeTime(token.revokedAt!) }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="loading" data-testid="workspace-tokens-loading">
      <DbSkeleton class="h-24 w-full" />
    </div>

    <DbModal
      :open="showCreate"
      title="New workspace token"
      @close="!creating && (showCreate = false)">
      <form class="flex flex-col gap-4" @submit.prevent="submitCreate">
        <DbInput v-model="newName" label="Token name" mono />
        <DbSelect
          v-model="expiryChoice"
          label="Expires"
          :options="tokenExpiryOptions" />
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
            :error="expiryError" />
          <DbSelect
            v-model="customUnit"
            label="Unit"
            :options="tokenExpiryUnits" />
        </div>
        <p v-if="createError" class="text-xs text-crit">{{ createError }}</p>
        <div class="flex justify-end gap-2">
          <DbButton variant="ghost" @click="showCreate = false">Cancel</DbButton>
          <DbButton variant="primary" type="submit" :loading="creating">
            Create token
          </DbButton>
        </div>
      </form>
    </DbModal>

    <OneTimeTokenDialog
      :id="created?.token.id ?? ''"
      :open="created !== null"
      title="Workspace token created"
      :name="created?.token.name ?? ''"
      :token="created?.plaintextToken ?? ''"
      :detail="
        created?.token.expiresAt
          ? `Expires ${formatDateTime(created.token.expiresAt)}.`
          : 'This token does not expire.'
      "
      @acknowledge="acknowledgeCreated" />

    <DbConfirmDialog
      :open="revokeTarget !== null"
      title="Revoke workspace token"
      :description="`Applications using '${revokeTarget?.name}' will lose runtime access to this workspace.`"
      :confirm-word="revokeTarget?.name"
      confirm-label="Revoke token"
      :loading="revokeLoading"
      :error="revokeError"
      @confirm="confirmRevoke"
      @close="revokeTarget = null" />
  </div>
</template>
