import { computed, ref } from "vue";
import { useQuery, useQueryCache } from "@pinia/colada";
import {
  createBackup,
  deleteBackup,
  downloadBackupUrl,
  downloadMasterKeyBlob,
  restoreBackup,
  uploadBackup,
  type BackupItem,
} from "~/services/backups.api";
import { useReauthentication } from "~/composable/reauthentication";
import { errorMessage, isAbortError } from "~/services/api-errors";
import { backupsQuery } from "~/queries/backups";
import { loadFailure } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { saveBlob } from "~/utils/download";

/**
 * Backups page controller: manages snapshot creation, listing, file upload,
 * point-in-time restoration, and deletion. Each row's ⋯ menu calls
 * `triggerDownload`, `openRestoreDialog` or `openDeleteDialog`.
 */
export function useBackupsController() {
  const { runWithReauth } = useReauthentication();
  const cache = useQueryCache();
  const invalidate = useInvalidate();
  const backupList = useQuery(backupsQuery);
  const backups = computed<BackupItem[]>(() => backupList.data.value ?? []);
  const loading = backupList.isLoading;
  /** Whether the list has answered once, so empty means empty. */
  const hasLoaded = computed(
    () =>
      backupList.data.value !== undefined || backupList.error.value !== null,
  );
  // Says so even above the old list, which may be out of date.
  const loadError = loadFailure(
    backupList,
    "Could not load the list of backups.",
    { withData: true },
  );

  /** Applies a change the server just confirmed, without asking again. */
  function patch(update: (list: BackupItem[]) => BackupItem[]): void {
    cache.setQueryData(backupsQuery.key, (list) => update(list ?? []));
  }
  const actionMessage = ref<{ text: string; tone: "ok" | "crit" } | null>(null);
  const downloadingMasterKey = ref(false);

  // Create modal state
  const createModalOpen = ref(false);
  const customName = ref("");
  const creating = ref(false);
  const createError = ref<string | null>(null);

  // Upload modal state
  const uploadModalOpen = ref(false);
  const selectedFile = ref<File | null>(null);
  const selectedKeyFile = ref<File | null>(null);
  const keyHex = ref("");
  const uploading = ref(false);
  const uploadError = ref<string | null>(null);

  // Restore dialog state
  const restoreTarget = ref<BackupItem | null>(null);
  const restoreKeyHex = ref("");
  const restoring = ref(false);
  const restoreError = ref<string | null>(null);

  // Delete dialog state
  const deleteTarget = ref<BackupItem | null>(null);
  const deleting = ref(false);
  const deleteError = ref<string | null>(null);

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await backupList.refetch();
  }

  function openCreateModal(): void {
    createModalOpen.value = true;
    customName.value = "";
    createError.value = null;
  }

  function closeCreateModal(): void {
    if (creating.value) return;
    createModalOpen.value = false;
  }

  async function submitCreate(): Promise<void> {
    creating.value = true;
    createError.value = null;
    try {
      const created = await createBackup(customName.value);
      patch((list) => [created, ...list.filter((b) => b.key !== created.key)]);
      createModalOpen.value = false;
      actionMessage.value = {
        text: `Backup "${created.key}" was created successfully.`,
        tone: "ok",
      };
    } catch (cause: unknown) {
      createError.value = errorMessage(
        cause,
        "Failed to create backup. Please try again.",
      );
    } finally {
      creating.value = false;
    }
  }

  function openUploadModal(): void {
    uploadModalOpen.value = true;
    selectedFile.value = null;
    selectedKeyFile.value = null;
    keyHex.value = "";
    uploadError.value = null;
  }

  function closeUploadModal(): void {
    if (uploading.value) return;
    uploadModalOpen.value = false;
  }

  function onFileSelected(file: File | null): void {
    selectedFile.value = file;
    uploadError.value = null;
  }

  function onKeyFileSelected(file: File | null): void {
    selectedKeyFile.value = file;
    if (file) keyHex.value = "";
    uploadError.value = null;
  }

  async function submitUpload(): Promise<void> {
    if (!selectedFile.value) {
      uploadError.value = "Please select a backup file (.dop) to upload.";
      return;
    }
    uploading.value = true;
    uploadError.value = null;
    try {
      const keyInput =
        selectedKeyFile.value ||
        (keyHex.value.trim() ? keyHex.value.trim() : undefined);
      const uploaded = keyInput
        ? await uploadBackup(selectedFile.value, keyInput)
        : await uploadBackup(selectedFile.value);
      patch((list) => [
        uploaded,
        ...list.filter((b) => b.key !== uploaded.key),
      ]);
      uploadModalOpen.value = false;
      actionMessage.value = {
        text: `Backup "${uploaded.key}" was uploaded and verified.`,
        tone: "ok",
      };
    } catch (cause: unknown) {
      uploadError.value = errorMessage(
        cause,
        "Failed to upload backup. When uploading from another server, provide its master key.",
      );
    } finally {
      uploading.value = false;
    }
  }

  function triggerDownload(key: string): void {
    const url = downloadBackupUrl(key);
    const link = document.createElement("a");
    link.href = url;
    link.setAttribute("download", key);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }

  async function triggerDownloadMasterKey(): Promise<void> {
    downloadingMasterKey.value = true;
    try {
      await runWithReauth(async () => {
        saveBlob(await downloadMasterKeyBlob(), "master.key");
      });
      actionMessage.value = {
        text: "Master key downloaded. Store it apart from your backups.",
        tone: "ok",
      };
    } catch (cause: unknown) {
      if (isAbortError(cause)) return;
      actionMessage.value = {
        text: errorMessage(cause, "Could not download the master key."),
        tone: "crit",
      };
    } finally {
      downloadingMasterKey.value = false;
    }
  }

  function openRestoreDialog(item: BackupItem): void {
    restoreTarget.value = item;
    restoreKeyHex.value = "";
    restoreError.value = null;
  }

  function closeRestoreDialog(): void {
    if (restoring.value) return;
    restoreTarget.value = null;
  }

  async function submitRestore(): Promise<void> {
    if (!restoreTarget.value) return;
    restoring.value = true;
    restoreError.value = null;
    try {
      await runWithReauth(async () => {
        if (restoreKeyHex.value.trim()) {
          await restoreBackup(
            restoreTarget.value!.key,
            restoreKeyHex.value.trim(),
          );
        } else {
          await restoreBackup(restoreTarget.value!.key);
        }
      });
      const restoredKey = restoreTarget.value.key;
      restoreTarget.value = null;
      actionMessage.value = {
        text: `System restored successfully from "${restoredKey}".`,
        tone: "ok",
      };
      // The whole database was replaced, so every screen reads it again.
      await invalidate.everythingChanged();
    } catch (cause: unknown) {
      restoreError.value = errorMessage(
        cause,
        "Failed to restore backup. Please verify credentials or master key.",
      );
    } finally {
      restoring.value = false;
    }
  }

  function openDeleteDialog(item: BackupItem): void {
    deleteTarget.value = item;
    deleteError.value = null;
  }

  function closeDeleteDialog(): void {
    if (deleting.value) return;
    deleteTarget.value = null;
  }

  async function submitDelete(): Promise<void> {
    if (!deleteTarget.value) return;
    deleting.value = true;
    deleteError.value = null;
    try {
      const targetKey = deleteTarget.value.key;
      await deleteBackup(targetKey);
      patch((list) => list.filter((b) => b.key !== targetKey));
      deleteTarget.value = null;
      actionMessage.value = {
        text: `Backup "${targetKey}" was deleted.`,
        tone: "ok",
      };
    } catch (cause: unknown) {
      deleteError.value = errorMessage(
        cause,
        "Failed to delete backup. Please try again.",
      );
    } finally {
      deleting.value = false;
    }
  }

  return {
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
    restoreKeyHex,
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
  };
}

export type BackupsController = ReturnType<typeof useBackupsController>;
