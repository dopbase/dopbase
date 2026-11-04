/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h } from "vue";
import { freshPinia } from "~/tests/pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as ssoApi from "~/services/sso.api";

vi.mock("vue-router", () => ({
  useRoute: () => ({ query: {} }),
  useRouter: () => ({ replace: vi.fn() }),
}));
vi.mock("~/services/sso.api");
vi.mock("~/services/auth.api");
vi.mock("~/layouts", async () => {
  const { defineComponent: define, h: render } = await import("vue");
  return {
    DashboardLayout: define({
      setup(_, { slots }) {
        return () => render("div", slots.default?.());
      },
    }),
  };
});

import SsoPage from "./Sso.page.vue";

const empty: ssoApi.SsoSettings = {
  provider: null,
  redirectUri: "https://dopbase.example.com/api/v1/auth/sso/callback",
  publicUrl: "https://dopbase.example.com",
  publicUrlSecure: true,
  presets: [
    {
      id: "google",
      label: "Google",
      buttonLabel: "Google",
      issuer: "https://accounts.google.com",
      hint: "Create an OAuth client of type Web application in Google Cloud.",
    },
    {
      id: "generic",
      label: "Other OpenID Connect provider",
      buttonLabel: "SSO",
      issuer: "https://idp.example.com",
      hint: "Use the issuer.",
    },
  ],
};

function configured(
  overrides: Partial<ssoApi.SsoProvider> = {},
): ssoApi.SsoSettings {
  return {
    ...empty,
    provider: {
      id: "sso_1",
      preset: "google",
      displayName: "Google",
      issuer: "https://accounts.google.com",
      clientId: "client",
      enabled: false,
      verifiedAt: null,
      linkedAccounts: 0,
      requireSso: false,
      hidePasswordForm: false,
      passwordlessAccounts: 0,
      updatedAt: "2026-10-24T10:00:00Z",
      ...overrides,
    },
  };
}

let app: ReturnType<typeof createApp> | null = null;

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

async function mountPage(settings: ssoApi.SsoSettings): Promise<HTMLElement> {
  vi.mocked(ssoApi.fetchSsoSettings).mockResolvedValue(settings);
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(SsoPage) });
  app.component(
    "RouterLink",
    defineComponent({
      props: { to: { type: [Object, String], required: true } },
      setup(props, { slots, attrs }) {
        return () =>
          h("a", { ...attrs, href: String(props.to) }, slots.default?.());
      },
    }),
  );
  app.mount(host);
  await settle();
  return host;
}

function find(host: HTMLElement, id: string): HTMLElement | null {
  return host.querySelector(`[data-testid="${id}"]`);
}

function button(host: HTMLElement, label: string): HTMLButtonElement | null {
  return (
    [...host.querySelectorAll("button")].find(
      (item) => item.textContent?.trim() === label,
    ) ?? null
  );
}

beforeEach(() => {
  freshPinia();
});

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

describe("Single sign-on page", () => {
  it("shows a status card that points to the form before setup", async () => {
    const host = await mountPage(empty);

    const summary = find(host, "sso-summary")!;
    expect(summary.textContent).toContain("No provider yet");
    expect(find(host, "sso-status")!.textContent?.trim()).toBe("Not set up");
    expect(find(host, "sso-summary-line")!.textContent).toContain(
      "Fill in the provider below and save.",
    );
    expect(find(host, "sso-test")).toBeNull();
    expect(find(host, "sso-toggle")).toBeNull();
    expect(find(host, "sso-save-note")!.textContent).toContain(
      "Saving checks the issuer.",
    );
    expect(button(host, "Save")).not.toBeNull();
    expect(find(host, "sso-options")).toBeNull();
    expect(find(host, "sso-remove")).toBeNull();
  });

  it("leads with the test sign-in while the provider is untested", async () => {
    const host = await mountPage(configured());

    expect(find(host, "sso-status")!.textContent?.trim()).toBe("Untested");
    expect(find(host, "sso-test")!.className).toContain("bg-accent");
    const toggle = find(host, "sso-toggle") as HTMLButtonElement;
    expect(toggle.textContent?.trim()).toBe("Turn on");
    expect(toggle.disabled).toBe(true);
    expect(find(host, "sso-save-note")!.textContent).toContain(
      "turns single sign-on off until the next test sign-in",
    );
    expect(button(host, "Save changes")).not.toBeNull();
    expect(find(host, "sso-options")).toBeNull();
    expect(find(host, "sso-remove")).not.toBeNull();
  });

  it("makes Turn on the main action after a good test", async () => {
    const host = await mountPage(
      configured({ verifiedAt: "2026-10-28T14:02:00Z" }),
    );

    expect(find(host, "sso-status")!.textContent?.trim()).toBe("Off");
    expect(find(host, "sso-test")!.className).not.toContain("bg-accent");
    const toggle = find(host, "sso-toggle") as HTMLButtonElement;
    expect(toggle.disabled).toBe(false);
    expect(toggle.className).toContain("bg-accent");
    expect(find(host, "sso-summary-line")!.textContent).toContain(
      "Last test sign-in",
    );
  });

  it("warns about accounts without a password once it is on", async () => {
    const host = await mountPage(
      configured({
        enabled: true,
        verifiedAt: "2026-10-28T14:02:00Z",
        linkedAccounts: 4,
        passwordlessAccounts: 2,
      }),
    );

    expect(find(host, "sso-status")!.textContent?.trim()).toBe("On");
    expect(find(host, "sso-summary-line")!.textContent).toContain(
      "4 people have linked their account.",
    );
    expect(find(host, "sso-toggle")!.textContent?.trim()).toBe("Turn off");
    const warning = find(host, "sso-passwordless-warning")!;
    expect(warning.textContent).toContain("2 accounts have no password.");
    expect(warning.querySelector("a")!.getAttribute("href")).toBe("/users");
    expect(find(host, "sso-options")).not.toBeNull();
  });

  it("locks the page behind the HTTPS notice", async () => {
    const host = await mountPage({ ...empty, publicUrlSecure: false });

    expect(find(host, "sso-blocked")).not.toBeNull();
    expect(host.querySelector("fieldset")!.disabled).toBe(true);
  });
});
