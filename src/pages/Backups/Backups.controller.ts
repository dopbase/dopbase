import { onMounted, ref } from "vue";
import {
  createBackup,
  deleteBackup,
  downloadBackupUrl,
  downloadMasterKeyBlob,
  fetchBackups,
  restoreBackup,
  uploadBackup,
  type BackupItem,
} from "~/services/backups.api";
import { useReauthentication } from "~/composable/reauthentication";

/**
 * Backups page controller: manages snapshot creation, listing, file upload,
 * point-in-time restoration, and deletion.
 */
export function useBackupsController() {
  const { runWithReauth } = useReauthentication();
  const backups = ref<BackupItem[]>([]);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
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

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      backups.value = await fetchBackups();
    } catch {
      loadError.value = "Could not load the list of backups.";
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);

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
      backups.value = [
        created,
        ...backups.value.filter((b) => b.key !== created.key),
      ];
      createModalOpen.value = false;
      actionMessage.value = {
        text: `Backup "${created.key}" was created successfully.`,
        tone: "ok",
      };
    } catch (err: any) {
      createError.value =
        err?.message || "Failed to create backup. Please try again.";
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
      backups.value = [
        uploaded,
        ...backups.value.filter((b) => b.key !== uploaded.key),
      ];
      uploadModalOpen.value = false;
      actionMessage.value = {
        text: `Backup "${uploaded.key}" was uploaded and verified.`,
        tone: "ok",
      };
    } catch (err: any) {
      uploadError.value =
        err?.message ||
        "Failed to upload backup. When uploading from another server, provide its master key.";
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
        const blob = await downloadMasterKeyBlob();
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.setAttribute("download", "master.key");
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);
        actionMessage.value = {
          text: "Master key downloaded. Keep this file safe and confidential.",
          tone: "ok",
        };
      });
    } catch (err: any) {
      if (err?.name === "AbortError") return;
      actionMessage.value = {
        text: err?.message || "Failed to download master key.",
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
      await load();
    } catch (err: any) {
      restoreError.value =
        err?.message ||
        "Failed to restore backup. Please verify credentials or master key.";
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
      backups.value = backups.value.filter((b) => b.key !== targetKey);
      deleteTarget.value = null;
      actionMessage.value = {
        text: `Backup "${targetKey}" was deleted.`,
        tone: "ok",
      };
    } catch (err: any) {
      deleteError.value =
        err?.message || "Failed to delete backup. Please try again.";
    } finally {
      deleting.value = false;
    }
  }

  return {
    backups,
    loading,
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
