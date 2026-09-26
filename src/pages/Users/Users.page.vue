<script setup lang="ts">
import { useUsersController } from "./Users.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbInput,
  DbModal,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import { KeyIcon, LockIcon, PlusIcon } from "~/assets/icons";
import { formatDateTime } from "~/utils/format";
import {
  MAX_TOKEN_EXPIRY_DAYS,
  MAX_TOKEN_EXPIRY_HOURS,
} from "~/utils/token-expiry";

const { state, actions } = useUsersController();
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-5xl p-8">
      <header class="mb-6 flex items-center justify-between">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">Users</h1>
          <p class="mt-1 text-sm text-ink-muted">
            Manage human access to this instance. The root account is protected.
          </p>
        </div>
        <DbButton size="sm" variant="primary" @click="actions.openCreate">
          <PlusIcon class="h-3.5 w-3.5" />
          Add User
        </DbButton>
      </header>

      <DbAlert v-if="state.error" class="mb-4">{{ state.error }}</DbAlert>

      <!-- Users list loading skeleton -->
      <div
        v-if="state.loading && !state.users.length"
        class="overflow-hidden rounded-card border border-line bg-panel"
        data-testid="users-skeleton">
        <div
          v-for="i in 3"
          :key="i"
          class="flex items-center gap-3 border-b border-line-soft px-4 py-3.5 last:border-0">
          <div class="min-w-0 flex-1 space-y-2">
            <DbSkeleton class="h-4 w-48" />
            <DbSkeleton class="h-3 w-20" />
          </div>
          <DbSkeleton class="h-7 w-12 rounded-control" />
          <DbSkeleton class="h-7 w-14 rounded-control" />
        </div>
      </div>

      <!-- Users list -->
      <div
        v-else
        class="overflow-hidden rounded-card border border-line bg-panel"
        data-testid="users-list">
        <div
          v-for="user in state.users"
          :key="user.id"
          class="flex items-center gap-3 border-b border-line-soft px-4 py-3 last:border-0">
          <div class="min-w-0 flex-1">
            <p class="truncate font-mono text-sm text-ink-strong">
              {{ user.email }}
            </p>
            <DbBadge :tone="user.role === 'root' ? 'accent' : 'neutral'">
              {{
                user.role === "root"
                  ? "Super admin (root)"
                  : user.role === "member"
                    ? "Member"
                    : "Admin"
              }}
            </DbBadge>
          </div>
          <DbButton
            v-if="user.role !== 'root'"
            size="sm"
            variant="secondary"
            @click="actions.openEdit(user)">
            Edit
          </DbButton>
          <DbButton
            v-if="user.role !== 'root'"
            size="sm"
            variant="danger"
            @click="actions.promptDelete(user)">
            Delete
          </DbButton>
        </div>
      </div>

      <!-- AI Agents section -->
      <section class="mt-8">
        <header class="mb-3 flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold text-ink-strong">AI agents</h2>
            <p class="text-xs text-ink-muted">
              Generate agent tokens with password confirmation.
            </p>
          </div>
          <DbButton
            size="sm"
            variant="secondary"
            @click="actions.openAgentCreate">
            Add AI agent
          </DbButton>
        </header>

        <!-- AI agents loading skeleton -->
        <div
          v-if="state.loading && !state.serviceAccounts.length"
          class="overflow-hidden rounded-card border border-line bg-panel"
          data-testid="agents-skeleton">
          <div
            v-for="i in 2"
            :key="i"
            class="flex items-center gap-3 border-b border-line-soft px-4 py-3.5 last:border-0">
            <div class="min-w-0 flex-1 space-y-2">
              <DbSkeleton class="h-4 w-36" />
              <DbSkeleton class="h-3 w-16" />
            </div>
            <DbSkeleton class="h-7 w-14 rounded-control" />
          </div>
        </div>

        <!-- AI agents list -->
        <div
          v-else
          class="overflow-hidden rounded-card border border-line bg-panel"
          data-testid="agents-list">
          <div
            v-for="account in state.serviceAccounts"
            :key="account.id"
            class="flex items-center gap-3 border-b border-line-soft px-4 py-3 last:border-0">
            <div class="min-w-0 flex-1">
              <p class="font-mono text-sm text-ink-strong">
                {{ account.name }}
              </p>
              <DbBadge tone="neutral">AI agent</DbBadge>
              <div class="mt-1 space-y-0.5 text-xs text-ink-muted">
                <p v-if="state.agentTokens[account.id] === null">
                  Could not load token details.
                </p>
                <p v-else-if="!state.agentTokens[account.id]?.length">
                  No active tokens
                </p>
                <p
                  v-for="token in state.agentTokens[account.id] ?? []"
                  :key="token.id">
                  {{ token.name }} ·
                  {{
                    token.expiresAt
                      ? `Expiry: ${formatDateTime(token.expiresAt)}`
                      : "No expiry"
                  }}
                </p>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <DbButton
                size="sm"
                variant="secondary"
                @click="actions.promptGetToken(account)">
                <KeyIcon class="h-3.5 w-3.5" />
                Get token
              </DbButton>
              <DbButton
                size="sm"
                variant="danger"
                @click="actions.promptDeleteAgent(account)">
                Delete
              </DbButton>
            </div>
          </div>
          <p
            v-if="!state.serviceAccounts.length"
            class="px-4 py-5 text-sm text-ink-muted">
            No AI agents configured.
          </p>
        </div>
      </section>

      <!-- Add / Edit User Modal -->
      <DbModal
        :open="state.showCreate"
        :title="state.editing ? 'Edit user' : 'Add User'"
        @close="actions.close">
        <div class="flex flex-col gap-3">
          <DbSelect
            v-model="state.role"
            label="Role"
            :options="state.roleOptions" />
          <DbAlert v-if="state.formError">{{ state.formError }}</DbAlert>
          <DbInput
            v-model="state.email"
            label="Email"
            type="email"
            autocomplete="email"
            placeholder="admin@example.com"
            :error="state.fieldErrors.email"
            required
            @input="state.fieldErrors.email = undefined" />
          <DbInput
            v-model="state.password"
            :label="state.editing ? 'New password (optional)' : 'Password'"
            type="password"
            autocomplete="new-password"
            :error="state.fieldErrors.password"
            :required="!state.editing"
            :hint="
              state.editing
                ? 'Leave blank to keep existing password.'
                : 'At least 12 characters.'
            "
            @input="state.fieldErrors.password = undefined" />
        </div>
        <template #footer>
          <DbButton variant="ghost" @click="actions.close">Cancel</DbButton>
          <DbButton
            :loading="state.saving"
            :disabled="!state.canSave || state.saving"
            @click="actions.save">
            Save
          </DbButton>
        </template>
      </DbModal>

      <!-- Add AI Agent Modal -->
      <DbModal
        :open="state.showAgentCreate"
        title="Add AI agent"
        @close="actions.closeAgentCreate">
        <div class="flex flex-col gap-3">
          <DbAlert v-if="state.agentFormError">{{
            state.agentFormError
          }}</DbAlert>
          <DbInput
            v-model="state.agentName"
            label="Name"
            placeholder="build-agent"
            :error="state.agentFieldError"
            required
            @input="state.agentFieldError = null" />
          <p class="text-xs text-ink-muted">
            The agent can read project metadata and secret names, but never
            secret values.
          </p>
        </div>
        <template #footer>
          <DbButton variant="ghost" @click="actions.closeAgentCreate"
            >Cancel</DbButton
          >
          <DbButton
            :loading="state.savingAgent"
            :disabled="!state.canSaveAgent || state.savingAgent"
            @click="actions.saveAgent">
            Create
          </DbButton>
        </template>
      </DbModal>

      <!-- Delete User Confirmation Dialog -->
      <DbConfirmDialog
        :open="state.userToDelete !== null"
        title="Delete user"
        :description="`Deleting '${state.userToDelete?.email}' permanently removes their access to this instance.`"
        :confirm-word="state.userToDelete?.email"
        confirm-label="Delete user"
        tone="danger"
        :loading="state.deletingUser"
        :error="state.deleteUserError"
        @confirm="actions.confirmDeleteUser"
        @close="actions.closeDeleteUser" />

      <!-- Delete AI Agent Confirmation Dialog -->
      <DbConfirmDialog
        :open="state.agentToDelete !== null"
        title="Delete AI agent"
        :description="`Deleting '${state.agentToDelete?.name}' permanently removes it. Its tokens stop working immediately.`"
        :confirm-word="state.agentToDelete?.name"
        confirm-label="Delete AI agent"
        tone="danger"
        :loading="state.deletingAgent"
        :error="state.deleteAgentError"
        @confirm="actions.confirmDeleteAgent"
        @close="actions.closeDeleteAgent" />

      <!-- AI Agent Token Re-authentication Modal -->
      <DbModal
        :open="state.showTokenReauth"
        title="Authenticate to generate token"
        size="sm"
        persistent
        @close="actions.closeTokenReauth">
        <form
          class="flex flex-col gap-4"
          novalidate
          @submit.prevent="actions.confirmTokenReauthAndGenerate">
          <div class="flex items-start gap-3">
            <div
              class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-warn/30 bg-warn/10 text-warn">
              <LockIcon class="h-4 w-4" />
            </div>
            <p class="text-sm text-ink">
              Enter your password to generate a new token for
              <span class="font-semibold text-ink-strong">{{
                state.agentForToken?.name
              }}</span
              >. Any existing active token for this agent will be revoked.
            </p>
          </div>

          <DbSelect
            v-model="state.tokenExpiryChoice"
            label="Expires"
            :options="state.tokenExpiryOptions" />
          <div
            v-if="state.tokenExpiryChoice === 'custom'"
            class="grid grid-cols-2 gap-2">
            <DbInput
              v-model="state.tokenCustomAmount"
              label="Duration"
              type="number"
              :min="1"
              :max="
                state.tokenCustomUnit === 'h'
                  ? MAX_TOKEN_EXPIRY_HOURS
                  : MAX_TOKEN_EXPIRY_DAYS
              "
              :step="1"
              :error="state.tokenExpiryError"
              @input="state.tokenExpiryError = null" />
            <DbSelect
              v-model="state.tokenCustomUnit"
              label="Unit"
              :options="state.tokenExpiryUnits" />
          </div>

          <DbInput
            v-model="state.tokenPassword"
            label="Password"
            type="password"
            autocomplete="current-password"
            name="token-password"
            :error="state.tokenPasswordError"
            required
            autofocus
            @input="state.tokenPasswordError = null" />

          <DbAlert v-if="state.tokenReauthError">{{
            state.tokenReauthError
          }}</DbAlert>

          <div class="flex items-center justify-end gap-2">
            <DbButton
              variant="ghost"
              :disabled="state.generatingToken"
              @click="actions.closeTokenReauth">
              Cancel
            </DbButton>
            <DbButton
              variant="primary"
              type="submit"
              :loading="state.generatingToken"
              :disabled="
                state.tokenPassword.length === 0 || state.generatingToken
              ">
              Generate token
            </DbButton>
          </div>
        </form>
      </DbModal>

      <OneTimeTokenDialog
        :id="state.createdAgentToken?.token.id ?? ''"
        :open="state.createdAgentToken !== null"
        title="AI agent token generated"
        :name="state.createdAgentToken?.token.name ?? ''"
        :token="state.createdAgentToken?.plaintextToken ?? ''"
        :detail="
          state.createdAgentToken?.token.expiresAt
            ? `Expires ${formatDateTime(state.createdAgentToken.token.expiresAt)}.`
            : 'This token does not expire.'
        "
        @acknowledge="actions.acknowledgeCreatedToken" />
    </div>
  </DashboardLayout>
</template>
