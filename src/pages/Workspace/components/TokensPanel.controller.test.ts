import { describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useTokensPanelController } from "./TokensPanel.controller";
import * as tokensApi from "~/services/tokens.api";
import { ApiError } from "~/services/http.client";

vi.mock("~/services/tokens.api");

const token = {
  id: "tok_1",
  environmentId: "env_1",
  name: "deploy",
  createdAt: "2026-08-28T00:00:00Z",
  lastUsedAt: null,
  revokedAt: null,
};

function makeController() {
  return useTokensPanelController(ref("env_1"));
}

describe("useTokensPanelController", () => {
  it("creates a token and exposes the plaintext exactly once", async () => {
    vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
    vi.mocked(tokensApi.createToken).mockResolvedValueOnce({
      token,
      plaintextToken: "dbs_secret",
    });
    const c = makeController();
    await c.create("deploy");
    expect(tokensApi.createToken).toHaveBeenCalledWith("env_1", {
      name: "deploy",
      role: "runner",
    });
    expect(c.created.value?.plaintextToken).toBe("dbs_secret");
    c.acknowledgeCreated();
    expect(c.created.value).toBeNull();
  });

  it("sends the fixed runner role", async () => {
    vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
    vi.mocked(tokensApi.createToken).mockResolvedValueOnce({
      token,
      plaintextToken: "dbs_x",
    });
    const c = makeController();
    await c.create("ci");
    expect(tokensApi.createToken).toHaveBeenCalledWith("env_1", {
      name: "ci",
      role: "runner",
    });
  });

  it("maps name conflicts to a friendly message", async () => {
    vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
    vi.mocked(tokensApi.createToken).mockRejectedValueOnce(
      new ApiError(409, { TOKEN_NAME_TAKEN: "taken" }),
    );
    const c = makeController();
    await expect(c.create("deploy")).rejects.toBeDefined();
    expect(c.actionError.value).toBe("A token with this name already exists.");
  });

  it("revokes and reloads", async () => {
    vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
    vi.mocked(tokensApi.revokeToken).mockResolvedValueOnce({
      ...token,
      revokedAt: "2026-08-28T01:00:00Z",
    });
    const c = makeController();
    await c.revoke(token);
    expect(tokensApi.revokeToken).toHaveBeenCalledWith("tok_1");
    await vi.waitFor(() => expect(c.tokens.value).toEqual([]));
  });
});
