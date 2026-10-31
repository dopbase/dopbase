import { describe, expect, it } from "vitest";
import { isAgent, roleOf, signInMethod, tokenState } from "./people";
import {
  maya,
  memberRole,
  reader,
  reviewBot,
  root,
  tokenFixture,
  userFixture,
  viewerRole,
} from "~/tests/people";

const NOW = Date.parse("2026-10-31T12:00:00Z");

describe("isAgent", () => {
  it("treats only the AI agent role as an agent", () => {
    expect(isAgent(reviewBot)).toBe(true);
    expect(isAgent(maya)).toBe(false);
    expect(isAgent(root)).toBe(false);
  });
});

describe("signInMethod", () => {
  it("shows a disabled account as disabled whatever it signs in with", () => {
    expect(signInMethod(userFixture({ status: "disabled" }))).toBe("disabled");
    expect(
      signInMethod(
        userFixture({
          status: "disabled",
          hasPassword: false,
          ssoLinked: true,
        }),
      ),
    ).toBe("disabled");
  });

  it("shows single sign-on for an account linked to it without a password", () => {
    expect(
      signInMethod(userFixture({ hasPassword: false, ssoLinked: true })),
    ).toBe("sso");
  });

  it("shows password when the account has one, even if single sign-on is linked too", () => {
    expect(signInMethod(maya)).toBe("password");
    expect(signInMethod(userFixture({ ssoLinked: true }))).toBe("password");
  });

  it("shows none for an account with neither a password nor single sign-on", () => {
    expect(signInMethod(userFixture({ hasPassword: false }))).toBe("none");
  });
});

describe("roleOf", () => {
  it("finds the role row behind an account", () => {
    expect(roleOf(reader, [memberRole, viewerRole])).toBe(viewerRole);
  });

  it("returns null when the roles list doesn't have it, as for root", () => {
    expect(roleOf(root, [memberRole, viewerRole])).toBeNull();
    expect(roleOf(maya, [])).toBeNull();
  });
});

describe("tokenState", () => {
  it("calls a token with a future expiry active", () => {
    expect(
      tokenState(tokenFixture({ expiresAt: "2026-11-01T00:00:00Z" }), NOW),
    ).toBe("active");
  });

  it("calls a token without an expiry active", () => {
    expect(tokenState(tokenFixture({ expiresAt: null }), NOW)).toBe("active");
  });

  it("calls a token expired from the moment its expiry passes", () => {
    expect(
      tokenState(tokenFixture({ expiresAt: "2026-10-31T12:00:00Z" }), NOW),
    ).toBe("expired");
    expect(
      tokenState(tokenFixture({ expiresAt: "2026-10-30T00:00:00Z" }), NOW),
    ).toBe("expired");
  });

  it("calls a revoked token revoked even after it expired", () => {
    expect(
      tokenState(
        tokenFixture({
          revokedAt: "2026-10-20T00:00:00Z",
          expiresAt: "2026-10-25T00:00:00Z",
        }),
        NOW,
      ),
    ).toBe("revoked");
  });
});
