<script setup lang="ts">
import { useRolesController } from "./Roles.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbSkeleton,
} from "~/components/ui";
import {
  CopyIcon,
  EyeIcon,
  InfoIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
} from "~/assets/icons";

/**
 * Roles lists every role: how deep into project data its holders reach,
 * what else they may do, and who holds it. Root only.
 */
const controller = useRolesController();
const { roles, error, rows, deleting, deleteBusy, deleteError } = controller;
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Roles"
        subtitle="A role sets how far into project data its holders can reach and what else they may do here. Changes apply on their next request.">
        <template #actions>
          <DbButton
            variant="primary"
            data-testid="new-role"
            @click="$router.push({ name: 'role-new' })">
            <PlusIcon class="h-4 w-4" />
            New role
          </DbButton>
        </template>
      </PageHeader>

      <DbAlert v-if="error">{{ error }}</DbAlert>

      <div v-if="roles === null" class="flex flex-col gap-2">
        <DbSkeleton
          v-for="line in 4"
          :key="line"
          class="h-12 w-full rounded-control" />
      </div>

      <template v-else>
        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table class="min-w-full text-left text-sm" data-testid="roles-table">
            <thead>
              <tr class="border-b border-line-soft bg-raised/50">
                <th class="h-11 px-4 font-medium text-ink-muted">Role</th>
                <th class="h-11 px-4 font-medium text-ink-muted">
                  Reaches at most
                </th>
                <th class="h-11 px-4 font-medium text-ink-muted">
                  Capabilities
                </th>
                <th class="h-11 px-4 font-medium text-ink-muted">Signs in</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Holders</th>
                <th class="h-11 w-12 px-2">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in rows"
                :key="row.role.id"
                class="border-b border-line-soft last:border-b-0"
                data-testid="role-row">
                <td class="h-[56px] max-w-80 px-4">
                  <span class="flex items-center gap-2">
                    <RouterLink
                      :to="row.to"
                      class="truncate font-medium text-ink-strong hover:underline">
                      {{ row.role.name }}
                    </RouterLink>
                    <span
                      class="shrink-0 rounded-control px-1.5 py-0.5 font-mono text-xs"
                      :class="
                        row.role.builtin
                          ? 'bg-raised text-ink-muted'
                          : 'bg-accent-soft text-accent-strong'
                      ">
                      {{ row.role.builtin ? "built-in" : "custom" }}
                    </span>
                  </span>
                  <p
                    v-if="row.role.description"
                    class="truncate text-xs text-ink-muted">
                    {{ row.role.description }}
                  </p>
                </td>
                <td class="px-4"><DbLevelBadge v-bind="row.reach" /></td>
                <td class="px-4 font-mono text-ink">
                  {{ row.capabilityCount }}
                </td>
                <td class="px-4 text-ink">{{ row.signsIn }}</td>
                <td class="px-4 font-mono text-ink">{{ row.holders }}</td>
                <td class="px-2 text-right">
                  <DbMenu :label="`Actions for ${row.role.name}`">
                    <DbMenuItem @select="$router.push(row.to)">
                      <template #icon>
                        <EyeIcon v-if="row.role.builtin" class="h-4 w-4" />
                        <PencilIcon v-else class="h-4 w-4" />
                      </template>
                      {{ row.role.builtin ? "View" : "Edit" }}
                    </DbMenuItem>
                    <DbMenuItem @select="controller.duplicate(row.role)">
                      <template #icon><CopyIcon class="h-4 w-4" /></template>
                      Duplicate
                    </DbMenuItem>
                    <template v-if="!row.role.builtin">
                      <div class="my-0.5 h-px bg-line-soft" role="separator" />
                      <DbMenuItem
                        tone="danger"
                        @select="controller.askDelete(row.role)">
                        <template #icon><TrashIcon class="h-4 w-4" /></template>
                        Delete role
                      </DbMenuItem>
                    </template>
                  </DbMenu>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <ul class="flex flex-col gap-2 sm:hidden" data-testid="roles-cards">
          <li
            v-for="row in rows"
            :key="row.role.id"
            class="rounded-card border border-line bg-panel p-3">
            <RouterLink :to="row.to" class="flex flex-col gap-1.5">
              <span
                class="flex items-center gap-2 text-sm font-medium text-ink-strong">
                {{ row.role.name }}
                <span class="font-mono text-xs font-normal text-ink-muted">
                  {{ row.role.builtin ? "built-in" : "custom" }}
                </span>
              </span>
              <span
                class="flex flex-wrap items-center gap-2 text-xs text-ink-muted">
                <DbLevelBadge v-bind="row.reach" />
                {{ row.signsIn }} · {{ row.holders }}
                {{ row.holders === 1 ? "holder" : "holders" }}
              </span>
            </RouterLink>
          </li>
        </ul>

        <p
          class="flex items-start gap-2 rounded-card border border-line bg-panel px-4 py-3 text-sm text-ink-muted">
          <InfoIcon class="mt-0.5 h-4 w-4 shrink-0" />
          Built-in roles can't be changed. Duplicate one to start a custom role.
        </p>
      </template>
    </div>

    <DbConfirmDialog
      :open="deleting !== null"
      :title="`Delete ${deleting?.name ?? 'the role'}?`"
      :description="
        deleting?.members
          ? `${deleting.members} ${deleting.members === 1 ? 'account holds' : 'accounts hold'} this role. Give them another role first.`
          : 'Nobody holds this role. Deleting it can\'t be undone.'
      "
      :confirm-word="deleting?.name"
      confirm-label="Delete role"
      :loading="deleteBusy"
      :error="deleteError"
      @confirm="controller.confirmDelete"
      @close="controller.cancelDelete" />
  </DashboardLayout>
</template>
