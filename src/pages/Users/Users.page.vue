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

const {
  users,
  serviceAccounts,
  loading,
  error,
  editing,
  email,
  password,
  role,
  roleOptions,
  showCreate,
  saving,
  formError,
  fieldErrors,
  canSave,
  openCreate,
  openEdit,
  save,
  promptDelete,
  userToDelete,
  deletingUser,
  deleteUserError,
  closeDeleteUser,
  confirmDeleteUser,
  close,
  showAgentCreate,
  agentName,
  savingAgent,
  agentFormError,
  agentFieldError,
  canSaveAgent,
  openAgentCreate,
  closeAgentCreate,
  saveAgent,
  promptDeleteAgent,
  agentToDelete,
  deletingAgent,
  deleteAgentError,
  closeDeleteAgent,
  confirmDeleteAgent,
  agentForToken,
  showTokenReauth,
  tokenPassword,
  tokenPasswordError,
  tokenReauthError,
  generatingToken,
  createdAgentToken,
  promptGetToken,
  closeTokenReauth,
  confirmTokenReauthAndGenerate,
  acknowledgeCreatedToken,
} = useUsersController();
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-4xl p-8">
      <header class="mb-6 flex items-center justify-between">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">Users</h1>
          <p class="mt-1 text-sm text-ink-muted">
            Manage human access to this instance. The root account is protected.
          </p>
        </div>
        <DbButton size="sm" variant="primary" @click="openCreate">
          <PlusIcon class="h-3.5 w-3.5" />
          Add User
        </DbButton>
      </header>

      <DbAlert v-if="error" class="mb-4">{{ error }}</DbAlert>

      <!-- Users list loading skeleton -->
      <div
        v-if="loading && !users.length"
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
          v-for="user in users"
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
            @click="openEdit(user)">
            Edit
          </DbButton>
          <DbButton
            v-if="user.role !== 'root'"
            size="sm"
            variant="danger"
            @click="promptDelete(user)">
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
              Metadata-only identities. Generate a 30-day bearer token with password confirmation.
            </p>
          </div>
          <DbButton size="sm" variant="secondary" @click="openAgentCreate">
            Add AI agent
          </DbButton>
        </header>

        <!-- AI agents loading skeleton -->
        <div
          v-if="loading && !serviceAccounts.length"
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
            v-for="account in serviceAccounts"
            :key="account.id"
            class="flex items-center gap-3 border-b border-line-soft px-4 py-3 last:border-0">
            <div class="min-w-0 flex-1">
              <p class="font-mono text-sm text-ink-strong">
                {{ account.name }}
              </p>
              <DbBadge tone="neutral">AI agent</DbBadge>
            </div>
            <div class="flex items-center gap-2">
              <DbButton
                size="sm"
                variant="secondary"
                @click="promptGetToken(account)">
                <KeyIcon class="h-3.5 w-3.5" />
                Get token
              </DbButton>
              <DbButton
                size="sm"
                variant="danger"
                @click="promptDeleteAgent(account)">
                Delete
              </DbButton>
            </div>
          </div>
          <p
            v-if="!serviceAccounts.length"
            class="px-4 py-5 text-sm text-ink-muted">
            No AI agents configured.
          </p>
        </div>
      </section>

      <!-- Add / Edit User Modal -->
      <DbModal
        :open="showCreate"
        :title="editing ? 'Edit user' : 'Add User'"
        @close="close">
        <div class="flex flex-col gap-3">
          <DbSelect v-model="role" label="Role" :options="roleOptions" />
          <DbAlert v-if="formError">{{ formError }}</DbAlert>
          <DbInput
            v-model="email"
            label="Email"
            type="email"
            autocomplete="email"
            placeholder="admin@example.com"
            :error="fieldErrors.email"
            required
            @input="fieldErrors.email = undefined" />
          <DbInput
            v-model="password"
            :label="editing ? 'New password (optional)' : 'Password'"
            type="password"
            autocomplete="new-password"
            :error="fieldErrors.password"
            :required="!editing"
            :hint="
              editing
                ? 'Leave blank to keep existing password.'
                : 'At least 12 characters.'
            "
            @input="fieldErrors.password = undefined" />
        </div>
        <template #footer>
          <DbButton variant="ghost" @click="close">Cancel</DbButton>
          <DbButton
            :loading="saving"
            :disabled="!canSave || saving"
            @click="save">
            Save
          </DbButton>
        </template>
      </DbModal>

      <!-- Add AI Agent Modal -->
      <DbModal
        :open="showAgentCreate"
        title="Add AI agent"
        @close="closeAgentCreate">
        <div class="flex flex-col gap-3">
          <DbAlert v-if="agentFormError">{{ agentFormError }}</DbAlert>
          <DbInput
            v-model="agentName"
            label="Name"
            placeholder="build-agent"
            :error="agentFieldError"
            required
            @input="agentFieldError = null" />
          <p class="text-xs text-ink-muted">
            The agent can read project metadata and secret names, but never
            secret values.
          </p>
        </div>
        <template #footer>
          <DbButton variant="ghost" @click="closeAgentCreate">Cancel</DbButton>
          <DbButton
            :loading="savingAgent"
            :disabled="!canSaveAgent || savingAgent"
            @click="saveAgent">
            Create
          </DbButton>
        </template>
      </DbModal>

      <!-- Delete User Confirmation Dialog -->
      <DbConfirmDialog
        :open="userToDelete !== null"
        title="Delete user"
        :description="`Deleting '${userToDelete?.email}' permanently removes their access to this instance.`"
        :confirm-word="userToDelete?.email"
        confirm-label="Delete user"
        tone="danger"
        :loading="deletingUser"
        :error="deleteUserError"
        @confirm="confirmDeleteUser"
        @close="closeDeleteUser" />

      <!-- Delete AI Agent Confirmation Dialog -->
      <DbConfirmDialog
        :open="agentToDelete !== null"
        title="Delete AI agent"
        :description="`Deleting '${agentToDelete?.name}' permanently removes it. Its tokens stop working immediately.`"
        :confirm-word="agentToDelete?.name"
        confirm-label="Delete AI agent"
        tone="danger"
        :loading="deletingAgent"
        :error="deleteAgentError"
        @confirm="confirmDeleteAgent"
        @close="closeDeleteAgent" />

      <!-- AI Agent Token Re-authentication Modal -->
      <DbModal
        :open="showTokenReauth"
        title="Authenticate to generate token"
        size="sm"
        persistent
        @close="closeTokenReauth">
        <form
          class="flex flex-col gap-4"
          novalidate
          @submit.prevent="confirmTokenReauthAndGenerate">
          <div class="flex items-start gap-3">
            <div
              class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-warn/30 bg-warn/10 text-warn">
              <LockIcon class="h-4 w-4" />
            </div>
            <p class="text-sm text-ink">
              Enter your password to generate a new token for
              <span class="font-semibold text-ink-strong">{{ agentForToken?.name }}</span>.
              Any existing active token for this agent will be revoked.
            </p>
          </div>

          <DbInput
            v-model="tokenPassword"
            label="Password"
            type="password"
            autocomplete="current-password"
            name="token-password"
            :error="tokenPasswordError"
            required
            autofocus
            @input="tokenPasswordError = null" />

          <DbAlert v-if="tokenReauthError">{{ tokenReauthError }}</DbAlert>

          <div class="flex items-center justify-end gap-2">
            <DbButton
              variant="ghost"
              :disabled="generatingToken"
              @click="closeTokenReauth">
              Cancel
            </DbButton>
            <DbButton
              variant="primary"
              type="submit"
              :loading="generatingToken"
              :disabled="tokenPassword.length === 0 || generatingToken">
              Generate token
            </DbButton>
          </div>
        </form>
      </DbModal>

      <OneTimeTokenDialog
        :id="createdAgentToken?.token.id ?? ''"
        :open="createdAgentToken !== null"
        title="AI agent token generated"
        :name="createdAgentToken?.token.name ?? ''"
        :token="createdAgentToken?.plaintextToken ?? ''"
        detail="This token expires in 30 days."
        @acknowledge="acknowledgeCreatedToken" />
    </div>
  </DashboardLayout>
</template>
