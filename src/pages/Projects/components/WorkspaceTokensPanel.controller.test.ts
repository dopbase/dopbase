import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useWorkspaceTokensPanelController } from "./WorkspaceTokensPanel.controller";
import * as tokensApi from "~/services/tokens.api";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/tokens.api");

describe("useWorkspaceTokensPanelController", () => {
  it("loads workspace tokens for the active workspace id", async () => {
    vi.mocked(tokensApi.listWorkspaceTokens).mockResolvedValue([
      {
        id: "wtk_1",
        workspaceId: "wsp_default",
        name: "ci",
        createdAt: "2026-01-01T00:00:00Z",
        expiresAt: null,
        lastUsedAt: null,
        revokedAt: null,
      },
    ]);
    const { controller } = mountController(() =>
      useWorkspaceTokensPanelController(ref("wsp_default")),
    );
    await vi.waitFor(() => expect(controller.tokens.value).toHaveLength(1));
    expect(tokensApi.listWorkspaceTokens).toHaveBeenCalledWith(
      "wsp_default",
      expect.any(AbortSignal),
    );
  });
});
