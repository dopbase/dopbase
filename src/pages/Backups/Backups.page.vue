<script setup lang="ts">
import { ref } from "vue";
import {
  ArchiveIcon,
  DownloadIcon,
  KeyIcon,
  PlusIcon,
  RefreshIcon,
  TrashIcon,
  UploadIcon,
} from "~/assets/icons";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCode,
  DbConfirmDialog,
  DbEmptyState,
  DbInput,
  DbModal,
  DbSkeleton,
} from "~/components/ui";
import { DashboardLayout } from "~/layouts";
import { formatBytes, formatDateTime, formatRelativeTime } from "~/utils";
import { useBackupsController } from "./Backups.controller";

/**
 * Backups page: encrypted full system snapshot management.
 * Users can create new snapshots, upload backups, restore, and download them.
 */
const {
  backups,
  loading,
  hasLoaded,
  loadError,
  actionMessage,
  downloadingMasterKey,
  load,
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
    <div class="mx-auto max-w-5xl p-8">
      <!-- Header -->
      <header class="mb-6 flex flex-wrap items-center justify-between gap-4">
        <div>
          <div class="flex items-center gap-2.5">
            <h1 class="text-lg font-semibold text-ink-strong">Backups</h1>
            <DbBadge v-if="hasLoaded" tone="neutral">
              {{ backups.length }}
              {{ backups.length === 1 ? "snapshot" : "snapshots" }}
            </DbBadge>
          </div>
          <p class="mt-0.5 text-sm text-ink-muted">
            Full system encrypted snapshots for disaster recovery.
          </p>
        </div>
        <div class="flex items-center gap-2">
          <DbButton
            size="sm"
            variant="secondary"
            :loading="loading"
            @click="load">
            <RefreshIcon class="h-3.5 w-3.5" />
            Refresh
          </DbButton>
          <DbButton size="sm" variant="secondary" @click="openUploadModal">
            <UploadIcon class="h-3.5 w-3.5" />
            Upload backup
          </DbButton>
          <DbButton size="sm" variant="primary" @click="openCreateModal">
            <PlusIcon class="h-3.5 w-3.5" />
            New backup
          </DbButton>
        </div>
      </header>

      <!-- Master Key Notice Card -->
      <div
        class="mb-6 flex flex-col gap-3 rounded-card border border-accent/25 bg-accent/5 p-4.5">
        <div class="flex items-start gap-3">
          <div
            class="flex h-9 w-9 shrink-0 items-center justify-center rounded-control border border-accent/30 bg-accent-soft text-accent-strong">
            <KeyIcon class="h-4 w-4" />
          </div>
          <div class="text-xs leading-relaxed">
            <p class="font-semibold text-ink-strong">Encryption Key Notice</p>
            <p class="mt-0.5 text-ink-muted">
              All snapshots are encrypted with this server's master key
              (<DbCode>~/.dopbase/master.key</DbCode>). Download and safeguard
              this key to decrypt and restore backups on a fresh server.
            </p>
          </div>
        </div>
        <div class="shrink-0 self-end">
          <DbButton
            size="sm"
            variant="secondary"
            :loading="downloadingMasterKey"
            title="Download this server's master encryption key (master.key)"
            @click="triggerDownloadMasterKey">
            <KeyIcon class="h-3.5 w-3.5" />
            Download Master Key
          </DbButton>
        </div>
      </div>

      <!-- Feedback alerts -->
      <DbAlert
        v-if="actionMessage"
        class="mb-4"
        :tone="actionMessage.tone === 'ok' ? 'success' : 'error'">
        {{ actionMessage.text }}
      </DbAlert>

      <DbAlert v-if="loadError" class="mb-4" tone="error">
        {{ loadError }}
      </DbAlert>

      <!-- Loading skeleton placeholder state -->
      <div
        v-if="loading && !hasLoaded"
        class="overflow-hidden rounded-card border border-line bg-panel shadow-sm"
        data-testid="backups-skeleton">
        <div class="overflow-x-auto">
          <table class="w-full table-fixed text-left text-sm">
            <thead>
              <tr
                class="border-b border-line bg-panel/50 text-xs font-medium text-ink-muted">
                <th class="px-5 py-3.5">Backup</th>
                <th class="w-28 px-5 py-3.5">Size</th>
                <th class="w-40 px-5 py-3.5">Created</th>
                <th class="w-56 px-5 py-3.5 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-line-soft">
              <tr v-for="i in 4" :key="i">
                <td class="px-5 py-3.5">
                  <div class="flex items-center gap-2.5">
                    <DbSkeleton class="h-4 w-4 shrink-0 rounded" />
                    <DbSkeleton class="h-4 w-48" />
                  </div>
                </td>
                <td class="w-28 px-5 py-3.5">
                  <DbSkeleton class="h-4 w-16" />
                </td>
                <td class="w-40 px-5 py-3.5">
                  <DbSkeleton class="h-4 w-24" />
                </td>
                <td class="w-56 px-5 py-3.5 text-right">
                  <div class="flex items-center justify-end gap-1">
                    <DbSkeleton class="h-8 w-24 rounded-control" />
                    <DbSkeleton class="h-8 w-20 rounded-control" />
                    <DbSkeleton class="h-8 w-8 rounded-control" />
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Empty state -->
      <DbEmptyState
        v-else-if="hasLoaded && backups.length === 0"
        title="No backups found"
        description="Create your first encrypted snapshot to safeguard projects, environments, and secrets.">
        <template #icon>
          <ArchiveIcon class="h-6 w-6" />
        </template>
        <template #actions>
          <DbButton size="sm" variant="primary" @click="openCreateModal">
            <PlusIcon class="h-3.5 w-3.5" />
            Create backup
          </DbButton>
          <DbButton size="sm" variant="secondary" @click="openUploadModal">
            <UploadIcon class="h-3.5 w-3.5" />
            Upload backup
          </DbButton>
        </template>
      </DbEmptyState>

      <!-- Backups Table -->
      <div
        v-else
        class="overflow-hidden rounded-card border border-line bg-panel shadow-sm"
        data-testid="backups-table">
        <div class="overflow-x-auto">
          <table class="w-full table-fixed text-left text-sm">
            <thead>
              <tr
                class="border-b border-line bg-panel/50 text-xs font-medium text-ink-muted">
                <th class="px-5 py-3.5">Backup</th>
                <th class="w-28 px-5 py-3.5">Size</th>
                <th class="w-40 px-5 py-3.5">Created</th>
                <th class="w-56 px-5 py-3.5 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-line-soft">
              <tr
                v-for="backup in backups"
                :key="backup.key"
                class="transition-colors hover:bg-raised/40">
                <td class="px-5 py-3.5">
                  <div class="flex items-center gap-2.5">
                    <ArchiveIcon class="h-4 w-4 shrink-0 text-ink-muted" />
                    <span
                      class="truncate font-mono text-xs font-semibold text-ink-strong"
                      :title="backup.key">
                      {{ backup.key }}
                    </span>
                  </div>
                </td>
                <td class="w-28 px-5 py-3.5 font-mono text-xs text-ink-muted">
                  {{ formatBytes(backup.size) }}
                </td>
                <td class="w-40 px-5 py-3.5 text-xs text-ink-muted">
                  <span :title="formatDateTime(backup.createdAt)">
                    {{ formatRelativeTime(backup.createdAt) }}
                  </span>
                </td>
                <td class="w-56 px-5 py-3.5 text-right">
                  <div class="flex items-center justify-end gap-1">
                    <DbButton
                      size="sm"
                      variant="ghost"
                      title="Download backup archive"
                      aria-label="Download backup"
                      @click="triggerDownload(backup.key)">
                      <DownloadIcon class="h-3.5 w-3.5" />
                      <span class="hidden sm:inline">Download</span>
                    </DbButton>
                    <DbButton
                      size="sm"
                      variant="ghost"
                      title="Restore system from this backup"
                      aria-label="Restore backup"
                      @click="openRestoreDialog(backup)">
                      <RefreshIcon class="h-3.5 w-3.5 text-accent" />
                      <span class="hidden text-accent sm:inline">Restore</span>
                    </DbButton>
                    <DbButton
                      size="sm"
                      variant="ghost"
                      title="Delete backup archive"
                      aria-label="Delete backup"
                      @click="openDeleteDialog(backup)">
                      <TrashIcon class="h-3.5 w-3.5 text-crit" />
                    </DbButton>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- Create Backup Modal -->
    <DbModal
      :open="createModalOpen"
      title="Create New Backup"
      size="sm"
      @close="closeCreateModal">
      <form class="flex flex-col gap-4" @submit.prevent="submitCreate">
        <p class="text-xs text-ink-muted">
          Creates a point-in-time snapshot of the database. The archive is
          encrypted with the server master key.
        </p>

        <DbInput
          v-model="customName"
          label="Backup name (optional)"
          placeholder="e.g. pre_deployment_v2"
          mono
          :error="createError"
          hint="Leave empty to use an automatic timestamped name." />

        <div
          class="mt-2 flex items-center justify-end gap-2 border-t border-line-soft pt-3">
          <DbButton
            variant="ghost"
            type="button"
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

    <!-- Upload Backup Modal -->
    <DbModal
      :open="uploadModalOpen"
      title="Upload Backup"
      size="md"
      @close="closeUploadModal">
      <div class="flex flex-col gap-4">
        <p class="text-xs text-ink-muted">
          Upload an encrypted
          <code class="font-mono text-ink-strong">.dop</code> backup file.
          Dopbase will verify its authentication tag before adding it to the
          available backups.
        </p>

        <input
          ref="fileInputRef"
          type="file"
          accept=".dop"
          class="hidden"
          @change="handleFileInputChange" />

        <div
          class="flex cursor-pointer flex-col items-center justify-center rounded-lg border border-dashed border-line p-6 text-center transition-colors hover:border-accent hover:bg-raised/20"
          @click="chooseFile">
          <UploadIcon class="h-8 w-8 text-ink-muted" />
          <p class="mt-2 text-sm font-medium text-ink-strong">
            {{
              selectedFile
                ? selectedFile.name
                : "Click to select a .dop backup file"
            }}
          </p>
          <p v-if="selectedFile" class="mt-1 font-mono text-xs text-ink-muted">
            {{ formatBytes(selectedFile.size) }}
          </p>
          <p v-else class="mt-1 text-xs text-ink-muted">
            Select an encrypted .dop backup file.
          </p>
        </div>

        <!-- Cross-Server Master Key option -->
        <div class="rounded-lg border border-line bg-panel/50 p-3">
          <p class="text-xs font-medium text-ink-strong">
            Cross-Server Master Key (optional)
          </p>
          <p class="mt-0.5 text-xs text-ink-muted">
            If this backup was created on a different Dopbase instance, provide
            its master key. Dopbase will safely re-key the snapshot using this
            server's master key so it can be restored seamlessly without
            changing this server's key.
          </p>
          <input
            ref="keyFileInputRef"
            type="file"
            class="hidden"
            @change="handleKeyFileInputChange" />
          <div class="mt-2 flex items-center gap-2">
            <DbButton
              variant="secondary"
              size="sm"
              type="button"
              @click="chooseKeyFile">
              {{
                selectedKeyFile ? "Change key file" : "Select master.key file"
              }}
            </DbButton>
            <span
              v-if="selectedKeyFile"
              class="font-mono text-xs text-ink-strong">
              {{ selectedKeyFile.name }} ({{
                formatBytes(selectedKeyFile.size)
              }})
            </span>
            <span v-else class="text-xs text-ink-faint">
              Or paste 64-char hex key below
            </span>
          </div>
          <DbInput
            v-if="!selectedKeyFile"
            v-model="keyHex"
            class="mt-2"
            label="Hex Master Key (optional)"
            name="keyHex"
            placeholder="e.g. 4a2f8b..."
            mono />
        </div>

        <DbAlert v-if="uploadError" tone="error">
          {{ uploadError }}
        </DbAlert>

        <div
          class="mt-2 flex items-center justify-end gap-2 border-t border-line-soft pt-3">
          <DbButton
            variant="ghost"
            type="button"
            :disabled="uploading"
            @click="closeUploadModal">
            Cancel
          </DbButton>
          <DbButton
            variant="primary"
            type="button"
            :disabled="!selectedFile"
            :loading="uploading"
            @click="submitUpload">
            Upload &amp; Verify
          </DbButton>
        </div>
      </div>
    </DbModal>

    <!-- Restore Confirmation Dialog -->
    <DbConfirmDialog
      :open="!!restoreTarget"
      title="Restore System Snapshot?"
      :description="`Restoring from &quot;${restoreTarget?.key}&quot; will overwrite all current projects, environments, and secrets with the snapshot data. Your current session will be preserved.`"
      confirm-label="Restore Snapshot"
      confirm-word="RESTORE"
      tone="danger"
      :loading="restoring"
      :error="restoreError"
      @confirm="submitRestore"
      @close="closeRestoreDialog" />

    <!-- Delete Confirmation Dialog -->
    <DbConfirmDialog
      :open="!!deleteTarget"
      title="Delete Backup?"
      :description="`Are you sure you want to permanently delete backup file &quot;${deleteTarget?.key}&quot;?`"
      confirm-label="Delete Backup"
      tone="danger"
      :loading="deleting"
      :error="deleteError"
      @confirm="submitDelete"
      @close="closeDeleteDialog" />
  </DashboardLayout>
</template>
