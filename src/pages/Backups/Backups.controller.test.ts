import { beforeEach, describe, expect, it, vi } from "vitest";
import { useQuery } from "@pinia/colada";
import { keys } from "~/queries/keys";
import { freshPinia } from "~/tests/pinia";
import { useBackupsController } from "./Backups.controller";
import * as backupsApi from "~/services/backups.api";
import { ApiError } from "~/services/http.client";
import { saveBlob } from "~/utils/download";
import { mountController } from "~/tests/mount-controller";
import { settle } from "~/tests/people";

vi.mock("~/services/backups.api");
vi.mock("~/utils/download");

const sampleBackup: backupsApi.BackupItem = {
  key: "backup_test.dop",
  size: 1024,
  createdAt: "2026-09-05T10:00:00Z",
};

beforeEach(() => {
  freshPinia();
  vi.clearAllMocks();
});

/** Opens the page with `list` on the server. */
async function mounted(list: backupsApi.BackupItem[] = []) {
  vi.mocked(backupsApi.fetchBackups).mockResolvedValueOnce(list);
  const { controller } = mountController(useBackupsController);
  await settle();
  return controller;
}

describe("useBackupsController", () => {
  it("loads backups successfully", async () => {
    const c = await mounted([sampleBackup]);

    expect(backupsApi.fetchBackups).toHaveBeenCalledTimes(1);
    expect(c.backups.value).toEqual([sampleBackup]);
    expect(c.loadError.value).toBeNull();
  });

  it("handles loading errors gracefully", async () => {
    vi.mocked(backupsApi.fetchBackups).mockRejectedValueOnce(
      new Error("Network failed"),
    );

    const { controller: c } = mountController(useBackupsController);
    await settle();

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

    const c = await mounted();
    c.openCreateModal();
    c.customName.value = "custom_name";

    await c.submitCreate();

    expect(backupsApi.createBackup).toHaveBeenCalledWith("custom_name");
    expect(c.backups.value[0]).toEqual(created);
    expect(c.createModalOpen.value).toBe(false);
    expect(c.actionMessage.value?.tone).toBe("ok");

    // Coming back shows it at once, with no new request.
    const again = mountController(useBackupsController).controller;
    expect(again.backups.value[0]).toEqual(created);
    expect(backupsApi.fetchBackups).toHaveBeenCalledTimes(1);
  });

  it("handles create error", async () => {
    vi.mocked(backupsApi.createBackup).mockRejectedValueOnce(
      new Error("Disk full"),
    );

    const c = await mounted();
    c.openCreateModal();

    await c.submitCreate();

    expect(c.createError.value).toBe("Disk full");
    expect(c.createModalOpen.value).toBe(true);
  });

  it("validates file before uploading", async () => {
    const c = await mounted();
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

      const c = await mounted();
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
    // Another screen's data, which the restore replaced too.
    const listProjects = vi.fn().mockResolvedValue([]);
    mountController(() =>
      useQuery({ key: keys.projects, query: listProjects }),
    );

    const c = await mounted();
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
    expect(listProjects).toHaveBeenCalledTimes(2);
  });

  it("deletes a backup and removes it from the list", async () => {
    vi.mocked(backupsApi.deleteBackup).mockResolvedValueOnce();

    const c = await mounted([sampleBackup]);
    c.openDeleteDialog(sampleBackup);

    await c.submitDelete();

    expect(backupsApi.deleteBackup).toHaveBeenCalledWith(sampleBackup.key);
    expect(c.backups.value).toEqual([]);
    expect(c.deleteTarget.value).toBeNull();
    expect(c.actionMessage.value?.tone).toBe("ok");
  });

  describe("row menu", () => {
    it("downloads the backup file from its own URL", async () => {
      vi.mocked(backupsApi.downloadBackupUrl).mockReturnValueOnce(
        "/api/v1/backups/backup_test.dop",
      );
      const click = vi
        .spyOn(HTMLAnchorElement.prototype, "click")
        .mockImplementation(() => {});

      const c = await mounted();
      c.triggerDownload(sampleBackup.key);

      expect(backupsApi.downloadBackupUrl).toHaveBeenCalledWith(
        sampleBackup.key,
      );
      const link = click.mock.contexts[0] as HTMLAnchorElement;
      expect(link.getAttribute("href")).toBe("/api/v1/backups/backup_test.dop");
      expect(link.getAttribute("download")).toBe(sampleBackup.key);
      expect(document.querySelector("a[download]")).toBeNull();
      click.mockRestore();
    });

    it("keeps the restore dialog open with the server's reason", async () => {
      vi.mocked(backupsApi.restoreBackup).mockRejectedValueOnce(
        new ApiError(400, {}, "This backup was made with another master key."),
      );

      const c = await mounted();
      c.openRestoreDialog(sampleBackup);
      await c.submitRestore();

      expect(c.restoreTarget.value).toEqual(sampleBackup);
      expect(c.restoreError.value).toBe(
        "This backup was made with another master key.",
      );
      // Only the first load: a failed restore changes nothing to reload.
      expect(backupsApi.fetchBackups).toHaveBeenCalledTimes(1);
    });

    it("keeps the backup listed when deleting fails", async () => {
      vi.mocked(backupsApi.deleteBackup).mockRejectedValueOnce(
        new ApiError(500, {}, "The file is in use."),
      );

      const c = await mounted([sampleBackup]);
      c.openDeleteDialog(sampleBackup);
      await c.submitDelete();

      expect(c.backups.value).toEqual([sampleBackup]);
      expect(c.deleteTarget.value).toEqual(sampleBackup);
      expect(c.deleteError.value).toBe("The file is in use.");
    });
  });

  it("saves the master key from the banner", async () => {
    const key = new Blob(["key"]);
    vi.mocked(backupsApi.downloadMasterKeyBlob).mockResolvedValueOnce(key);

    const c = await mounted();
    await c.triggerDownloadMasterKey();

    expect(saveBlob).toHaveBeenCalledWith(key, "master.key");
    expect(c.actionMessage.value).toEqual({
      text: "Master key downloaded. Store it apart from your backups.",
      tone: "ok",
    });
    expect(c.downloadingMasterKey.value).toBe(false);
  });
});
