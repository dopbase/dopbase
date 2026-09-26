import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useBackupsController } from "./Backups.controller";
import * as backupsApi from "~/services/backups.api";

vi.mock("vue", async (importOriginal) => ({
  ...(await importOriginal<typeof import("vue")>()),
  onMounted: vi.fn(),
}));

vi.mock("~/services/backups.api");

const sampleBackup: backupsApi.BackupItem = {
  key: "backup_test.dop",
  size: 1024,
  createdAt: "2026-09-05T10:00:00Z",
};

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

describe("useBackupsController", () => {
  it("loads backups successfully", async () => {
    vi.mocked(backupsApi.fetchBackups).mockResolvedValueOnce([sampleBackup]);

    const c = useBackupsController();
    await c.load();

    expect(backupsApi.fetchBackups).toHaveBeenCalledTimes(1);
    expect(c.backups.value).toEqual([sampleBackup]);
    expect(c.loadError.value).toBeNull();
  });

  it("handles loading errors gracefully", async () => {
    vi.mocked(backupsApi.fetchBackups).mockRejectedValueOnce(
      new Error("Network failed"),
    );

    const c = useBackupsController();
    await c.load();

    expect(c.backups.value).toEqual([]);
    expect(c.loadError.value).toBe("Could not load the list of backups.");
  });

  it("creates a backup and updates the list", async () => {
    const created = {
      key: "custom_name.dop",
      size: 2048,
      createdAt: "2026-09-05T10:05:00Z",
    };
    vi.mocked(backupsApi.createBackup).mockResolvedValueOnce(created);

    const c = useBackupsController();
    c.openCreateModal();
    c.customName.value = "custom_name";

    await c.submitCreate();

    expect(backupsApi.createBackup).toHaveBeenCalledWith("custom_name");
    expect(c.backups.value[0]).toEqual(created);
    expect(c.createModalOpen.value).toBe(false);
    expect(c.actionMessage.value?.tone).toBe("ok");
  });

  it("handles create error", async () => {
    vi.mocked(backupsApi.createBackup).mockRejectedValueOnce(
      new Error("Disk full"),
    );

    const c = useBackupsController();
    c.openCreateModal();

    await c.submitCreate();

    expect(c.createError.value).toBe("Disk full");
    expect(c.createModalOpen.value).toBe(true);
  });

  it("validates file before uploading", async () => {
    const c = useBackupsController();
    c.openUploadModal();

    await c.submitUpload();

    expect(c.uploadError.value).toBe(
      "Please select a backup file (.dop) to upload.",
    );
    expect(backupsApi.uploadBackup).not.toHaveBeenCalled();
  });

  it.each([{ withKey: false }, { withKey: true }])(
    "uploads a backup and adds it to the list (master key file: $withKey)",
    async ({ withKey }) => {
      const uploaded = {
        key: "uploaded.dop",
        size: 4096,
        createdAt: "2026-09-05T10:10:00Z",
      };
      vi.mocked(backupsApi.uploadBackup).mockResolvedValueOnce(uploaded);

      const c = useBackupsController();
      c.openUploadModal();
      const fakeFile = new File(["test"], "uploaded.dop");
      const fakeKey = new File(["key-data"], "master.key");
      c.onFileSelected(fakeFile);
      if (withKey) c.onKeyFileSelected(fakeKey);

      await c.submitUpload();

      expect(backupsApi.uploadBackup).toHaveBeenCalledWith(
        fakeFile,
        ...(withKey ? [fakeKey] : []),
      );
      expect(c.backups.value[0]).toEqual(uploaded);
      expect(c.uploadModalOpen.value).toBe(false);
      expect(c.actionMessage.value?.tone).toBe("ok");
    },
  );

  it.each([
    { label: "without a master key", keyHex: undefined },
    {
      label: "with an explicit hex master key",
      keyHex:
        "  0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  ",
    },
  ])("restores a backup $label and triggers a refresh", async ({ keyHex }) => {
    vi.mocked(backupsApi.restoreBackup).mockResolvedValueOnce();
    vi.mocked(backupsApi.fetchBackups).mockResolvedValueOnce([sampleBackup]);

    const c = useBackupsController();
    c.openRestoreDialog(sampleBackup);
    if (keyHex !== undefined) c.restoreKeyHex.value = keyHex;

    await c.submitRestore();

    expect(backupsApi.restoreBackup).toHaveBeenCalledWith(
      sampleBackup.key,
      ...(keyHex === undefined ? [] : [keyHex.trim()]),
    );
    expect(c.restoreTarget.value).toBeNull();
    expect(c.actionMessage.value?.tone).toBe("ok");
    expect(backupsApi.fetchBackups).toHaveBeenCalled();
  });

  it("deletes a backup and removes it from the list", async () => {
    vi.mocked(backupsApi.deleteBackup).mockResolvedValueOnce();

    const c = useBackupsController();
    c.backups.value = [sampleBackup];
    c.openDeleteDialog(sampleBackup);

    await c.submitDelete();

    expect(backupsApi.deleteBackup).toHaveBeenCalledWith(sampleBackup.key);
    expect(c.backups.value).toEqual([]);
    expect(c.deleteTarget.value).toBeNull();
    expect(c.actionMessage.value?.tone).toBe("ok");
  });
});
