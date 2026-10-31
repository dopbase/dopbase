<script setup lang="ts">
import { computed, ref } from "vue";
import {
  CAPABILITY_LABELS,
  ROLE_CAPABILITIES,
  useRolesController,
} from "./Roles.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCheckbox,
  DbConfirmDialog,
  DbInput,
  DbModal,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import { PlusIcon } from "~/assets/icons";
import { errorMessage } from "~/services/api-errors";
import type { Capability, Level } from "~/services/access.api";
import type { Role, SignIn, TokenScope } from "~/services/roles.api";
import { formatLevel } from "~/utils/format";

/**
 * Roles lists every role. Root creates custom roles from scratch, such as a
 * Viewer that reads values but never changes them, and changes or deletes
 * them later. Built-in roles stay as they are.
 */
const controller = useRolesController();
const { roles, error } = controller;

const LEVEL_OPTIONS = (
  ["metadata", "read", "write", "manage", "project_manager"] as Level[]
).map((value) => ({ value, label: formatLevel(value) }));
const TOKEN_LEVEL_OPTIONS = LEVEL_OPTIONS.slice(0, 3);
const SIGN_IN_OPTIONS = [
  { value: "interactive", label: "People who sign in" },
  {
    value: "token_only",
    label: "Accounts that only use tokens, like AI agents",
  },
];
const TOKEN_SCOPE_OPTIONS = [
  { value: "data", label: "Project data" },
  { value: "audit", label: "The audit log only" },
  { value: "none", label: "No tokens" },
];

const editing = ref<Role | null>(null);
const showForm = ref(false);
const name = ref("");
const description = ref("");
const capabilities = ref<Capability[]>([]);
const maxLevel = ref<Level>("read");
const signIn = ref<SignIn>("interactive");
const tokenScope = ref<TokenScope>("data");
const tokenMaxLevel = ref<Level>("read");
const tokenDefaultTtlDays = ref("30");
const tokenMaxTtlDays = ref("365");
const saving = ref(false);
const formError = ref<string | null>(null);
const deleteTarget = ref<Role | null>(null);
const deleting = ref(false);
const deleteError = ref<string | null>(null);

const holdsTokens = computed(() =>
  capabilities.value.includes("tokens.personal"),
);

function open(role: Role | null): void {
  editing.value = role;
  name.value = role?.name ?? "";
  description.value = role?.description ?? "";
  capabilities.value = role ? [...role.capabilities] : ["tokens.personal"];
  maxLevel.value = role?.maxLevel ?? "read";
  signIn.value = role?.signIn ?? "interactive";
  tokenScope.value = role?.tokenScope ?? "data";
  tokenMaxLevel.value = role?.tokenMaxLevel ?? "read";
  tokenDefaultTtlDays.value = String(role?.tokenDefaultTtlDays ?? 30);
  tokenMaxTtlDays.value = String(role?.tokenMaxTtlDays ?? 365);
  formError.value = null;
  showForm.value = true;
}

function toggle(capability: Capability, on: boolean): void {
  capabilities.value = on
    ? [...capabilities.value, capability]
    : capabilities.value.filter((item) => item !== capability);
}

async function submit(): Promise<void> {
  saving.value = true;
  formError.value = null;
  try {
    await controller.save(
      {
        name: name.value.trim(),
        description: description.value.trim(),
        capabilities: capabilities.value,
        maxLevel: maxLevel.value,
        signIn: signIn.value,
        tokenScope: holdsTokens.value ? tokenScope.value : "none",
        tokenMaxLevel: tokenMaxLevel.value,
        tokenDefaultTtlDays: Number(tokenDefaultTtlDays.value),
        tokenMaxTtlDays: Number(tokenMaxTtlDays.value),
      },
      editing.value?.id ?? null,
    );
    showForm.value = false;
  } catch (cause) {
    formError.value = errorMessage(cause, "Could not save the role.");
  } finally {
    saving.value = false;
  }
}

async function confirmDelete(): Promise<void> {
  if (!deleteTarget.value) return;
  deleting.value = true;
  deleteError.value = null;
  try {
    await controller.remove(deleteTarget.value);
    deleteTarget.value = null;
  } catch (cause) {
    deleteError.value = errorMessage(cause, "Could not delete the role.");
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <DashboardLayout>
    <div class="page-width py-6 lg:py-8">
      <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">Roles</h1>
          <p class="mt-1 max-w-2xl text-sm text-ink-muted">
            A role says what its holders may do on this instance and how deep
            into project data their grants can reach. Changes apply on their
            next request. Built-in roles can't change; create your own for
            anything else.
          </p>
        </div>
        <DbButton
          size="sm"
          variant="primary"
          data-testid="new-role"
          @click="open(null)">
          <PlusIcon class="h-3.5 w-3.5" />
          New role
        </DbButton>
      </header>

      <DbAlert v-if="error" class="mb-4">{{ error }}</DbAlert>

      <div v-if="roles === null" class="space-y-2">
        <DbSkeleton v-for="i in 4" :key="i" class="h-16 w-full rounded-card" />
      </div>

      <div
        v-else
        class="overflow-hidden rounded-card border border-line bg-panel"
        data-testid="roles-list">
        <div
          v-for="role in roles"
          :key="role.id"
          class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-line-soft px-4 py-3 last:border-0">
          <div class="min-w-0 flex-1 basis-full sm:basis-0">
            <div class="flex flex-wrap items-center gap-1.5">
              <p class="text-sm font-medium text-ink-strong">
                {{ role.name }}
              </p>
              <DbBadge v-if="role.builtin">built-in</DbBadge>
              <DbBadge v-if="role.signIn === 'token_only'" tone="accent">
                tokens only
              </DbBadge>
            </div>
            <p v-if="role.description" class="text-xs text-ink-muted">
              {{ role.description }}
            </p>
            <p class="mt-0.5 text-xs text-ink-faint">
              Up to {{ formatLevel(role.maxLevel) }} ·
              {{ role.capabilities.length }} capabilities ·
              {{ role.members }}
              {{ role.members === 1 ? "account" : "accounts" }}
            </p>
          </div>
          <template v-if="!role.builtin">
            <DbButton size="sm" variant="secondary" @click="open(role)">
              Edit
            </DbButton>
            <DbButton
              size="sm"
              variant="danger"
              @click="((deleteError = null), (deleteTarget = role))">
              Delete
            </DbButton>
          </template>
        </div>
      </div>
    </div>

    <DbModal
      :open="showForm"
      :title="editing ? `Edit ${editing.name}` : 'New role'"
      size="lg"
      @close="showForm = false">
      <form class="flex flex-col gap-4" @submit.prevent="submit">
        <div class="grid gap-3 sm:grid-cols-2">
          <DbInput v-model="name" label="Name" placeholder="e.g. Viewer" />
          <DbSelect
            :model-value="maxLevel"
            label="Reaches at most"
            :options="LEVEL_OPTIONS"
            @update:model-value="
              (value: string) => (maxLevel = value as Level)
            " />
        </div>
        <DbInput
          v-model="description"
          label="Description (optional)"
          placeholder="What the role is for" />
        <DbSelect
          v-if="!editing"
          :model-value="signIn"
          label="Holders"
          :options="SIGN_IN_OPTIONS"
          @update:model-value="(value: string) => (signIn = value as SignIn)" />

        <fieldset class="flex flex-col gap-2">
          <legend class="mb-1 text-xs font-semibold text-ink-muted">
            Holders can
          </legend>
          <DbCheckbox
            v-for="capability in ROLE_CAPABILITIES"
            :key="capability"
            :model-value="capabilities.includes(capability)"
            :label="CAPABILITY_LABELS[capability] ?? capability"
            @update:model-value="(on: boolean) => toggle(capability, on)" />
        </fieldset>

        <div v-if="holdsTokens" class="grid gap-3 sm:grid-cols-2">
          <DbSelect
            :model-value="tokenScope"
            label="Their tokens read"
            :options="TOKEN_SCOPE_OPTIONS.slice(0, 2)"
            @update:model-value="
              (value: string) => (tokenScope = value as TokenScope)
            " />
          <DbSelect
            v-if="tokenScope === 'data'"
            :model-value="tokenMaxLevel"
            label="Tokens reach at most"
            :options="TOKEN_LEVEL_OPTIONS"
            @update:model-value="
              (value: string) => (tokenMaxLevel = value as Level)
            " />
          <DbInput
            v-model="tokenDefaultTtlDays"
            label="Tokens last, by default (days)"
            type="number" />
          <DbInput
            v-model="tokenMaxTtlDays"
            label="Tokens last at most (days)"
            type="number" />
        </div>

        <p v-if="formError" class="text-xs text-crit">{{ formError }}</p>
        <div class="flex justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="saving"
            @click="showForm = false">
            Cancel
          </DbButton>
          <DbButton variant="primary" type="submit" :loading="saving">
            {{ editing ? "Save" : "Create role" }}
          </DbButton>
        </div>
      </form>
    </DbModal>

    <DbConfirmDialog
      :open="deleteTarget !== null"
      title="Delete role"
      :description="`Delete '${deleteTarget?.name}'? Accounts must hold another role first.`"
      :confirm-word="deleteTarget?.name"
      confirm-label="Delete role"
      :loading="deleting"
      :error="deleteError"
      @confirm="confirmDelete"
      @close="deleteTarget = null" />
  </DashboardLayout>
</template>
