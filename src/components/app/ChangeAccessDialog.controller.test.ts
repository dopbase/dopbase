import { beforeEach, describe, expect, it, vi } from "vitest";
import { freshPinia } from "~/tests/pinia";
import { nextTick, ref } from "vue";
import { useChangeAccessDialog } from "./ChangeAccessDialog.controller";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import type { GrantPrincipal } from "~/utils/level";
import { grantFixture } from "~/tests/people";

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  setGrant: vi.fn(),
}));

const maya: GrantPrincipal = {
  type: "user",
  id: "usr_maya",
  label: "maya@dopbase.com",
  maxLevel: "project_manager",
};
const bot: GrantPrincipal = {
  type: "user",
  id: "usr_bot",
  label: "review-bot",
  maxLevel: "read",
};

const prod = grantFixture({
  scopeType: "environment",
  scopeId: "env_prod",
  scopeLabel: "payments/prod",
  level: "read",
});

beforeEach(() => {
  freshPinia();
  vi.clearAllMocks();
  vi.mocked(accessApi.setGrant).mockResolvedValue(grantFixture());
});

describe("useChangeAccessDialog", () => {
  it("changes only the level of the grant", async () => {
    const dialog = useChangeAccessDialog(ref(maya), ref(prod));
    expect(dialog.level.value).toBe("read");
    expect(dialog.levels.value.map((choice) => choice.level)).not.toContain(
      "project_manager",
    );
    dialog.level.value = "metadata";
    expect(dialog.summary.value).toBe(
      "maya@dopbase.com will be able to see the secret names in payments/prod.",
    );
    expect(await dialog.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_maya",
      scopeType: "environment",
      scope: "env_prod",
      level: "metadata",
    });
  });

  it("says a project grant covers all of the project", () => {
    const dialog = useChangeAccessDialog(ref(maya), ref(grantFixture()));
    expect(dialog.summary.value).toMatch(/all of payments\.$/);
  });

  it("shows levels above an agent's role but won't give them", async () => {
    const dialog = useChangeAccessDialog(ref(bot), ref(grantFixture()));
    expect(
      dialog.levels.value.find((choice) => choice.level === "write")?.disabled,
    ).toBe(true);
    dialog.level.value = "write";
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("Their role allows up to Read.");
    expect(accessApi.setGrant).not.toHaveBeenCalled();
  });

  it("shows the server's reason when the change is refused", async () => {
    vi.mocked(accessApi.setGrant).mockRejectedValueOnce(
      new ApiError(403, {
        ACCESS_EXCEEDS_YOURS: "You can't give more access than you have.",
      }),
    );
    const dialog = useChangeAccessDialog(ref(maya), ref(prod));
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe(
      "You can't give more access than you have.",
    );
    expect(dialog.saving.value).toBe(false);
  });

  it("starts from the grant's level each time it opens", async () => {
    const grant = ref<accessApi.Grant | null>(null);
    const dialog = useChangeAccessDialog(ref(maya), grant);
    grant.value = prod;
    await nextTick();
    dialog.level.value = "write";
    grant.value = null;
    await nextTick();
    grant.value = { ...prod };
    await nextTick();
    expect(dialog.level.value).toBe("read");
  });
});
