<script setup lang="ts">
import { ref } from "vue";
import {
  DownloadIcon,
  KeyIcon,
  PlusIcon,
  RefreshIcon,
  TrashIcon,
  UploadIcon,
} from "~/assets/icons";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbInput,
  DbMenu,
  DbMenuItem,
  DbModal,
  DbSkeleton,
} from "~/components/ui";
import PageHeader from "~/components/app/PageHeader.vue";
import { DashboardLayout } from "~/layouts";
import { formatBytes, formatDateTime } from "~/utils";
import { useBackupsController } from "./Backups.controller";

/**
 * Backups: encrypted snapshots of the database. Download, Restore and
 * Delete sit in each row's ⋯ menu; the master key banner stays on top
 * because no backup restores elsewhere without it.
 */
const {
  backups,
  loading,
  hasLoaded,
  loadError,
  actionMessage,
  downloadingMasterKey,
  // Create
  createModalOpen,
  customName,
  creating,
  createError,
  openCreateModal,
  closeCreateModal,
  submitCreate,
  // Upload
  uploadModalOpen,
  selectedFile,
  selectedKeyFile,
  keyHex,
  uploading,
  uploadError,
  openUploadModal,
  closeUploadModal,
  onFileSelected,
  onKeyFileSelected,
  submitUpload,
  // Download
  triggerDownload,
  triggerDownloadMasterKey,
  // Restore
  restoreTarget,
  restoring,
  restoreError,
  openRestoreDialog,
  closeRestoreDialog,
  submitRestore,
  // Delete
  deleteTarget,
  deleting,
  deleteError,
  openDeleteDialog,
  closeDeleteDialog,
  submitDelete,
} = useBackupsController();

const fileInputRef = ref<HTMLInputElement | null>(null);
const keyFileInputRef = ref<HTMLInputElement | null>(null);

function handleFileInputChange(event: Event): void {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    onFileSelected(target.files[0]);
  }
}

function chooseFile(): void {
  fileInputRef.value?.click();
}

function handleKeyFileInputChange(event: Event): void {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    onKeyFileSelected(target.files[0]);
  }
}

function chooseKeyFile(): void {
  keyFileInputRef.value?.click();
}
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Backups"
        subtitle="Encrypted snapshots of the database. Restoring needs the master key that made them.">
        <template #actions>
          <DbButton data-testid="upload-backup" @click="openUploadModal">
            <UploadIcon class="h-4 w-4" />
            Upload
          </DbButton>
          <DbButton
            variant="primary"
            data-testid="create-backup"
            @click="openCreateModal">
            <PlusIcon class="h-4 w-4" />
            Create backup
          </DbButton>
        </template>
      </PageHeader>

      <!-- Master key banner -->
      <div
        class="flex flex-col gap-3 rounded-card bg-warn/15 px-4 py-3 sm:flex-row sm:items-center"
        data-testid="master-key-banner">
        <div class="flex min-w-0 flex-1 items-start gap-3 sm:items-center">
          <KeyIcon class="mt-0.5 h-4 w-4 shrink-0 text-warn sm:mt-0" />
          <div class="min-w-0 text-sm">
            <p class="font-semibold text-ink-strong">
              Keep the master key with your backups
            </p>
            <p class="text-ink">
              Without it, no backup can be restored on another server.
            </p>
          </div>
        </div>
        <DbButton
          class="w-full shrink-0 sm:w-auto"
          :loading="downloadingMasterKey"
          @click="triggerDownloadMasterKey">
          <DownloadIcon class="h-4 w-4" />
          Download master key
        </DbButton>
      </div>

      <DbAlert
        v-if="actionMessage"
        :tone="actionMessage.tone === 'ok' ? 'success' : 'error'">
        {{ actionMessage.text }}
      </DbAlert>
      <DbAlert v-if="loadError">{{ loadError }}</DbAlert>

      <div
        v-if="loading && !hasLoaded"
        class="flex flex-col gap-2"
        data-testid="backups-skeleton">
        <DbSkeleton
          v-for="line in 4"
          :key="line"
          class="h-12 w-full rounded-control" />
      </div>

      <div
        v-else-if="hasLoaded && backups.length === 0 && !loadError"
        class="rounded-card border border-dashed border-line px-6 py-10 text-center">
        <p class="text-sm font-medium text-ink-strong">No backups yet</p>
        <p class="mt-1 text-sm text-ink-muted">
          Create one before an upgrade or a big change, or upload one from
          another server.
        </p>
      </div>

      <template v-else-if="backups.length > 0">
        <!-- Table on wider screens -->
        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table
            class="w-full table-fixed text-left text-sm"
            data-testid="backups-table">
            <thead>
              <tr class="border-b border-line-soft bg-raised/50">
                <th class="h-11 px-4 font-medium text-ink-muted">Backup</th>
                <th class="h-11 w-28 px-4 font-medium text-ink-muted">Size</th>
                <th class="h-11 w-48 px-4 font-medium text-ink-muted">
                  Created
                </th>
                <th class="h-11 w-12 px-2">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="backup in backups"
                :key="backup.key"
                class="border-b border-line-soft last:border-b-0"
                data-testid="backup-row">
                <td class="h-[52px] px-4">
                  <p
                    class="truncate font-mono text-ink-strong"
                    :title="backup.key">
                    {{ backup.key }}
                  </p>
                </td>
                <td class="px-4 text-ink">{{ formatBytes(backup.size) }}</td>
                <td class="px-4 text-ink">
                  {{ formatDateTime(backup.createdAt) }}
                </td>
                <td class="px-2 text-right">
                  <DbMenu :label="`Actions for ${backup.key}`">
                    <DbMenuItem @select="triggerDownload(backup.key)">
                      <template #icon>
                        <DownloadIcon class="h-4 w-4" />
                      </template>
                      Download
                    </DbMenuItem>
                    <DbMenuItem @select="openRestoreDialog(backup)">
                      <template #icon><RefreshIcon class="h-4 w-4" /></template>
                      Restore
                    </DbMenuItem>
                    <div class="my-0.5 h-px bg-line-soft" role="separator" />
                    <DbMenuItem
                      tone="danger"
                      @select="openDeleteDialog(backup)">
                      <template #icon><TrashIcon class="h-4 w-4" /></template>
                      Delete backup
                    </DbMenuItem>
                  </DbMenu>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Cards on phones -->
        <ul class="flex flex-col gap-2 sm:hidden" data-testid="backups-cards">
          <li
            v-for="backup in backups"
            :key="backup.key"
            class="flex items-start gap-2 rounded-card border border-line bg-panel p-3">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <span
                class="font-mono text-sm text-ink-strong [overflow-wrap:anywhere]">
                {{ backup.key }}
              </span>
              <span class="text-xs text-ink-muted">
                {{ formatBytes(backup.size) }} ·
                {{ formatDateTime(backup.createdAt) }}
              </span>
            </div>
            <DbMenu :label="`Actions for ${backup.key}`">
              <DbMenuItem @select="triggerDownload(backup.key)">
                <template #icon><DownloadIcon class="h-4 w-4" /></template>
                Download
              </DbMenuItem>
              <DbMenuItem @select="openRestoreDialog(backup)">
                <template #icon><RefreshIcon class="h-4 w-4" /></template>
                Restore
              </DbMenuItem>
              <div class="my-0.5 h-px bg-line-soft" role="separator" />
              <DbMenuItem tone="danger" @select="openDeleteDialog(backup)">
                <template #icon><TrashIcon class="h-4 w-4" /></template>
                Delete backup
              </DbMenuItem>
            </DbMenu>
          </li>
        </ul>
      </template>
    </div>

    <!-- Create -->
    <DbModal
      :open="createModalOpen"
      title="Create backup"
      size="sm"
      @close="closeCreateModal">
      <form class="flex flex-col gap-4" @submit.prevent="submitCreate">
        <p class="text-sm text-ink-muted">
          Takes a snapshot of the database now, encrypted with this server's
          master key.
        </p>

        <DbInput
          v-model="customName"
          label="Name (optional)"
          placeholder="before-upgrade"
          mono
          :error="createError"
          hint="Leave it empty to name it by date and time." />

        <div
          class="mt-2 flex items-center justify-end gap-2 border-t border-line-soft pt-3">
          <DbButton
            variant="ghost"
            :disabled="creating"
            @click="closeCreateModal">
            Cancel
          </DbButton>
          <DbButton variant="primary" type="submit" :loading="creating">
            Create backup
          </DbButton>
        </div>
      </form>
    </DbModal>

    <!-- Upload -->
    <DbModal
      :open="uploadModalOpen"
      title="Upload backup"
      size="md"
      @close="closeUploadModal">
      <div class="flex flex-col gap-4">
        <p class="text-sm text-ink-muted">
          Upload an encrypted
          <code class="font-mono text-ink-strong">.dop</code> backup. Dopbase
          checks it can be decrypted before adding it to the list.
        </p>

        <input
          ref="fileInputRef"
          type="file"
          accept=".dop"
          class="hidden"
          @change="handleFileInputChange" />

        <button
          type="button"
          class="flex cursor-pointer flex-col items-center justify-center rounded-card border border-dashed border-line p-6 text-center transition-colors hover:border-accent hover:bg-raised/20"
          @click="chooseFile">
          <UploadIcon class="h-6 w-6 text-ink-muted" />
          <span class="mt-2 break-all text-sm font-medium text-ink-strong">
            {{ selectedFile ? selectedFile.name : "Choose a .dop file" }}
          </span>
          <span
            v-if="selectedFile"
            class="mt-1 font-mono text-xs text-ink-muted">
            {{ formatBytes(selectedFile.size) }}
          </span>
        </button>

        <div class="rounded-card border border-line p-3">
          <p class="text-sm font-medium text-ink-strong">
            Master key from another server (optional)
          </p>
          <p class="mt-0.5 text-sm text-ink-muted">
            If another Dopbase server made this backup, add its master key.
            Dopbase re-encrypts the backup with this server's key, which stays
            the same.
          </p>
          <input
            ref="keyFileInputRef"
            type="file"
            class="hidden"
            @change="handleKeyFileInputChange" />
          <div class="mt-2 flex flex-wrap items-center gap-2">
            <DbButton size="sm" @click="chooseKeyFile">
              {{ selectedKeyFile ? "Change key file" : "Choose master.key" }}
            </DbButton>
            <span
              v-if="selectedKeyFile"
              class="font-mono text-xs text-ink-strong">
              {{ selectedKeyFile.name }} ({{
                formatBytes(selectedKeyFile.size)
              }})
            </span>
            <span v-else class="text-xs text-ink-faint">
              or paste the 64-character hex key below
            </span>
          </div>
          <DbInput
            v-if="!selectedKeyFile"
            v-model="keyHex"
            class="mt-2"
            label="Hex master key (optional)"
            name="keyHex"
            placeholder="4a2f8b…"
            mono />
        </div>

        <DbAlert v-if="uploadError">{{ uploadError }}</DbAlert>

        <div
          class="mt-2 flex items-center justify-end gap-2 border-t border-line-soft pt-3">
          <DbButton
            variant="ghost"
            :disabled="uploading"
            @click="closeUploadModal">
            Cancel
          </DbButton>
          <DbButton
            variant="primary"
            :disabled="!selectedFile"
            :loading="uploading"
            @click="submitUpload">
            Upload
          </DbButton>
        </div>
      </div>
    </DbModal>

    <DbConfirmDialog
      :open="!!restoreTarget"
      title="Restore this backup?"
      :description="`Everything in ${restoreTarget?.key ?? 'the backup'} replaces the current projects, environments and secrets. You stay signed in.`"
      confirm-label="Restore"
      confirm-word="RESTORE"
      tone="danger"
      :loading="restoring"
      :error="restoreError"
      @confirm="submitRestore"
      @close="closeRestoreDialog" />

    <DbConfirmDialog
      :open="!!deleteTarget"
      :title="`Delete ${deleteTarget?.key ?? 'the backup'}?`"
      description="The file is removed from this server. Copies you downloaded aren't affected."
      :confirm-word="deleteTarget?.key"
      confirm-label="Delete backup"
      tone="danger"
      :loading="deleting"
      :error="deleteError"
      @confirm="submitDelete"
      @close="closeDeleteDialog" />
  </DashboardLayout>
</template>
