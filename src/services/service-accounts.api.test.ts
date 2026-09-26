import { beforeEach, describe, expect, it, vi } from "vitest";
import { createAgentToken } from "./service-accounts.api";
import { apiRequest } from "./http.client";

vi.mock("./http.client", () => ({ apiRequest: vi.fn() }));

describe("createAgentToken", () => {
  beforeEach(() => {
    vi.mocked(apiRequest).mockResolvedValue({ data: {} } as never);
  });

  it("keeps the existing third argument as an absolute timestamp", async () => {
    await createAgentToken("aia_1", "deploy", "2027-01-01T00:00:00Z");
    expect(apiRequest).toHaveBeenCalledWith(
      "/api/v1/service-accounts/aia_1/tokens",
      {
        method: "POST",
        body: {
          name: "deploy",
          expiresAt: "2027-01-01T00:00:00Z",
          expiresIn: undefined,
        },
      },
    );
  });

  it("sends a relative duration in the fourth argument", async () => {
    await createAgentToken("aia_1", "deploy", undefined, "7d");
    expect(apiRequest).toHaveBeenCalledWith(
      "/api/v1/service-accounts/aia_1/tokens",
      {
        method: "POST",
        body: { name: "deploy", expiresAt: undefined, expiresIn: "7d" },
      },
    );
  });
});
