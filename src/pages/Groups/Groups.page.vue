<script setup lang="ts">
import { computed, ref } from "vue";
import { useGroupsController } from "./Groups.controller";
import { DashboardLayout } from "~/layouts";
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
import { PlusIcon, ShieldIcon } from "~/assets/icons";
import { errorMessage } from "~/services/api-errors";
import { formatLevel } from "~/utils/format";

/**
 * Groups lists the groups on this instance. Opening one shows its members
 * and the grants every member receives through it. Grants themselves are
 * given on each project's Access page.
 */
const controller = useGroupsController();
const { groups, loadError, open, detailError, canManage, candidates } =
  controller;

const showCreate = ref(false);
const name = ref("");
const description = ref("");
const creating = ref(false);
const createError = ref<string | null>(null);
const newMember = ref("");
const confirmDelete = ref(false);
const deleting = ref(false);
const deleteError = ref<string | null>(null);

const candidateOptions = computed(() =>
  candidates.value.map((user) => ({ value: user.id, label: user.email })),
);

function openCreate(): void {
  name.value = "";
  description.value = "";
  createError.value = null;
  showCreate.value = true;
}

async function submitCreate(): Promise<void> {
  if (!name.value.trim()) {
    createError.value = "Enter a group name.";
    return;
  }
  creating.value = true;
  createError.value = null;
  try {
    await controller.create(name.value.trim(), description.value.trim());
    showCreate.value = false;
  } catch (cause) {
    createError.value = errorMessage(cause, "Could not create the group.");
  } finally {
    creating.value = false;
  }
}

async function showGroup(id: string): Promise<void> {
  newMember.value = "";
  await controller.show(id);
  newMember.value = candidateOptions.value[0]?.value ?? "";
}

async function addMember(): Promise<void> {
  if (!newMember.value) return;
  await controller.addMember(newMember.value);
  newMember.value = candidateOptions.value[0]?.value ?? "";
}

async function submitDelete(): Promise<void> {
  deleting.value = true;
  deleteError.value = null;
  try {
    await controller.remove();
    confirmDelete.value = false;
  } catch (cause) {
    deleteError.value = errorMessage(cause, "Could not delete the group.");
  } finally {
    deleting.value = false;
  }
}
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-5xl px-4 py-6 sm:px-6 lg:p-8">
      <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="text-lg font-semibold text-ink-strong">Groups</h1>
          <p class="mt-1 max-w-2xl text-sm text-ink-muted">
            Give a group access on a project's Access page, and everyone in the
            group gets it. Leaving a group takes that access away on the next
            request.
          </p>
        </div>
        <DbButton
          v-if="canManage"
          size="sm"
          variant="primary"
          data-testid="new-group"
          @click="openCreate">
          <PlusIcon class="h-3.5 w-3.5" />
          New group
        </DbButton>
      </header>

      <DbAlert v-if="loadError" class="mb-4">{{ loadError }}</DbAlert>

      <div v-if="groups === null" class="space-y-2">
        <DbSkeleton v-for="i in 3" :key="i" class="h-14 w-full rounded-card" />
      </div>

      <DbEmptyState
        v-else-if="groups.length === 0"
        title="No groups yet"
        description="Groups save you from giving the same access to people one at a time.">
        <template #icon>
          <ShieldIcon class="h-5 w-5" />
        </template>
        <template v-if="canManage" #actions>
          <DbButton variant="primary" size="sm" @click="openCreate">
            New group
          </DbButton>
        </template>
      </DbEmptyState>

      <div
        v-else
        class="overflow-hidden rounded-card border border-line bg-panel"
        data-testid="groups-list">
        <button
          v-for="group in groups"
          :key="group.id"
          type="button"
          class="flex w-full cursor-pointer items-center gap-3 border-b border-line-soft px-4 py-3 text-left transition-colors last:border-0 hover:bg-raised/60"
          @click="showGroup(group.id)">
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-ink-strong">
              {{ group.name }}
            </p>
            <p v-if="group.description" class="truncate text-xs text-ink-muted">
              {{ group.description }}
            </p>
          </div>
          <DbBadge>
            {{ group.memberCount }}
            {{ group.memberCount === 1 ? "member" : "members" }}
          </DbBadge>
        </button>
      </div>
    </div>

    <DbModal :open="showCreate" title="New group" @close="showCreate = false">
      <form class="flex flex-col gap-4" @submit.prevent="submitCreate">
        <DbInput v-model="name" label="Name" placeholder="e.g. Backend team" />
        <DbInput
          v-model="description"
          label="Description (optional)"
          placeholder="What the group is for" />
        <p v-if="createError" class="text-xs text-crit">{{ createError }}</p>
        <div class="flex justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="creating"
            @click="showCreate = false">
            Cancel
          </DbButton>
          <DbButton variant="primary" type="submit" :loading="creating">
            Create
          </DbButton>
        </div>
      </form>
    </DbModal>

    <DbModal
      :open="open !== null"
      :title="open?.group.name ?? ''"
      size="lg"
      @close="controller.close">
      <div v-if="open" class="flex flex-col gap-6">
        <DbAlert v-if="detailError">{{ detailError }}</DbAlert>

        <section>
          <h3 class="mb-2 text-sm font-medium text-ink-strong">Members</h3>
          <ul
            class="overflow-hidden rounded-card border border-line"
            data-testid="group-members">
            <li
              v-for="member in open.members"
              :key="member.id"
              class="flex items-center gap-3 border-b border-line-soft px-3 py-2 last:border-0">
              <span class="min-w-0 flex-1 truncate font-mono text-sm">
                {{ member.email }}
              </span>
              <DbButton
                v-if="canManage"
                size="sm"
                variant="ghost"
                :aria-label="`Remove ${member.email}`"
                @click="controller.removeMember(member.id)">
                Remove
              </DbButton>
            </li>
            <li
              v-if="open.members.length === 0"
              class="px-3 py-2 text-sm text-ink-muted">
              Nobody is in this group yet.
            </li>
          </ul>
          <form
            v-if="canManage && candidateOptions.length > 0"
            class="mt-3 flex items-end gap-2"
            @submit.prevent="addMember">
            <div class="min-w-0 flex-1">
              <DbSelect
                v-model="newMember"
                label="Add a member"
                :options="candidateOptions" />
            </div>
            <DbButton type="submit" variant="secondary">Add</DbButton>
          </form>
        </section>

        <section>
          <h3 class="mb-2 text-sm font-medium text-ink-strong">Access</h3>
          <ul
            v-if="open.grants.length > 0"
            class="overflow-hidden rounded-card border border-line">
            <li
              v-for="grant in open.grants"
              :key="grant.id"
              class="flex items-center gap-3 border-b border-line-soft px-3 py-2 last:border-0">
              <span class="min-w-0 flex-1 truncate font-mono text-sm">
                {{ grant.scopeLabel || "All projects" }}
              </span>
              <DbBadge>{{ formatLevel(grant.level) }}</DbBadge>
            </li>
          </ul>
          <p v-else class="text-sm text-ink-muted">
            This group has no access yet. Add it from a project's Access page.
          </p>
        </section>

        <div v-if="canManage" class="flex justify-end">
          <DbButton
            variant="danger"
            size="sm"
            @click="((deleteError = null), (confirmDelete = true))">
            Delete group
          </DbButton>
        </div>
      </div>
    </DbModal>

    <DbConfirmDialog
      :open="confirmDelete"
      title="Delete group"
      :description="`Everyone in '${open?.group.name}' loses the access the group gives them.`"
      :confirm-word="open?.group.name"
      confirm-label="Delete group"
      :loading="deleting"
      :error="deleteError"
      @confirm="submitDelete"
      @close="confirmDelete = false" />
  </DashboardLayout>
</template>
