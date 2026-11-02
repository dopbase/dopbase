/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Level } from "~/services/access.api";

const { controller } = await vi.hoisted(async () => {
  const { computed, ref } = await import("vue");
  const secrets = ref<{ key: string }[]>([]);
  const filter = ref("");
  return {
    controller: {
      secrets,
      filter,
      visibleSecrets: computed(() =>
        secrets.value.filter((secret) => secret.key.includes(filter.value)),
      ),
      copiedKey: ref(null),
      copyValue: vi.fn(),
      loading: ref(false),
      loadError: ref(null),
      actionError: ref(null),
      revealedKey: ref(null),
      revealedValue: ref(null),
      revealCountdown: ref(0),
      editorContent: ref(""),
      editorLoading: ref(false),
      editorLoadError: ref(null),
      editorAwaitingReauth: ref(false),
      editorSaving: ref(false),
      editorError: ref(null),
      editorDiff: ref(null),
      editorIssues: ref([]),
      editorDirty: ref(false),
      editorCanSave: ref(false),
      hideRevealed: vi.fn(),
      reveal: vi.fn(),
      openEditor: vi.fn(),
      closeEditor: vi.fn(),
      saveDraft: vi.fn(),
      applyDraft: vi.fn(),
      discardDraft: vi.fn(),
      backToEditing: vi.fn(),
      reload: vi.fn(),
      setSecret: vi.fn(async () => undefined),
      deleteSecret: vi.fn(async () => undefined),
    },
  };
});

vi.mock("./SecretsPanel.controller", () => ({
  useSecretsPanelController: () => controller,
}));
vi.mock("./ImportSecretsDialog.controller", async () => {
  const { ref } = await import("vue");
  return {
    useImportSecretsDialogController: () => ({
      parsing: ref(false),
      processFile: vi.fn(),
    }),
  };
});
const routerPush = vi.hoisted(() => vi.fn());
vi.mock("vue-router", () => ({
  useRoute: () => ({ name: "environment", query: {}, params: {} }),
  useRouter: () => ({ push: routerPush }),
}));
const empty = await vi.hoisted(async () => {
  const { ref } = await import("vue");
  return {
    view: ref<"loading" | "first-run" | "start">("first-run"),
    sources: ref<unknown[] | null>([]),
  };
});
vi.mock("./EmptyEnvironment.controller", () => ({
  useEmptyEnvironmentController: () => empty,
}));
const copyDialog = vi.hoisted(() => ({
  open: false,
  source: null as { name: string } | null,
  withValues: false,
}));
vi.mock("./CopySecretsDialog.vue", async () => {
  const { defineComponent: define } = await import("vue");
  return {
    default: define({
      props: {
        open: Boolean,
        source: { type: Object, default: null },
        withValues: Boolean,
      },
      setup(props) {
        return () => {
          copyDialog.open = props.open;
          copyDialog.source = props.source as { name: string } | null;
          copyDialog.withValues = props.withValues;
          return null;
        };
      },
    }),
  };
});
vi.mock("./EnvFileEditor.vue", () => ({
  default: defineComponent({ render: () => null }),
}));
vi.mock("./ImportSecretsDialog.vue", () => ({
  default: defineComponent({ render: () => null }),
}));
vi.mock("./ExportSecretsDialog.vue", () => ({
  default: defineComponent({ render: () => null }),
}));
const history = vi.hoisted(() => ({ key: null as string | null }));
vi.mock("./SecretHistoryDialog.vue", async () => {
  const { defineComponent: define } = await import("vue");
  return {
    default: define({
      props: { secretKey: { type: String, default: null } },
      setup(props) {
        return () => {
          history.key = props.secretKey;
          return null;
        };
      },
    }),
  };
});

import SecretsPanel from "./SecretsPanel.vue";

let app: ReturnType<typeof createApp> | null = null;

function render(level?: Level, canOpenAccess = false) {
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({
    render: () =>
      h(SecretsPanel, {
        environmentId: "env_1",
        environmentName: "production",
        environmentNames: ["dev", "production"],
        projectName: "payments",
        level,
        canOpenAccess,
      }),
  });
  app.mount(host);
  return host;
}

function button(text: string): HTMLButtonElement | undefined {
  return Array.from(document.querySelectorAll("button")).find(
    (candidate) => candidate.textContent?.trim() === text,
  );
}

/** Mounts an empty environment and opens Add secret from the checklist. */
function mountPanel() {
  render();
  button("Add secret")?.click();
  return nextTick();
}

const API_KEY = {
  key: "API_KEY",
  version: 3,
  createdAt: "2026-10-01T00:00:00Z",
  updatedAt: "2026-10-23T00:00:00Z",
};

async function openRowMenu(key: string): Promise<HTMLElement> {
  document
    .querySelector<HTMLButtonElement>(
      `button[aria-label="Actions for ${key}"]`,
    )!
    .click();
  await nextTick();
  return document.querySelector<HTMLElement>('[role="menu"]')!;
}

function inputValue(
  element: HTMLInputElement | HTMLTextAreaElement,
  value: string,
) {
  element.value = value;
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  controller.setSecret.mockClear();
  controller.copyValue.mockClear();
  controller.secrets.value = [];
  controller.filter.value = "";
  empty.view.value = "first-run";
  empty.sources.value = [];
  routerPush.mockClear();
});

describe("SecretsPanel create dialog", () => {
  it("requires a value unless empty value is selected", async () => {
    await mountPanel();
    inputValue(
      document.querySelector<HTMLInputElement>('input[name="key"]')!,
      "EMPTY_OK",
    );

    document.querySelector<HTMLFormElement>("form")?.requestSubmit();
    await nextTick();

    expect(controller.setSecret).not.toHaveBeenCalled();
    expect(document.body.textContent).toContain(
      "Enter a value or select Use empty value.",
    );
  });

  it("disables the field for an empty value and restores an earlier draft", async () => {
    await mountPanel();
    const key = document.querySelector<HTMLInputElement>('input[name="key"]')!;
    const value = document.querySelector<HTMLTextAreaElement>(
      'textarea[name="value"]',
    )!;
    const checkbox = document.querySelector<HTMLInputElement>(
      'input[type="checkbox"]',
    )!;
    inputValue(key, "OPTIONAL_VALUE");
    inputValue(value, "draft-secret");

    checkbox.click();
    await nextTick();
    expect(value.disabled).toBe(true);
    expect(value.value).toBe("");

    checkbox.click();
    await nextTick();
    expect(value.disabled).toBe(false);
    expect(value.value).toBe("draft-secret");

    checkbox.click();
    await nextTick();
    document.querySelector<HTMLFormElement>("form")?.requestSubmit();
    await nextTick();

    expect(controller.setSecret).toHaveBeenCalledWith("OPTIONAL_VALUE", "");
  });

  it("keeps whitespace secret values unchanged", async () => {
    await mountPanel();
    inputValue(
      document.querySelector<HTMLInputElement>('input[name="key"]')!,
      "SPACES",
    );
    inputValue(
      document.querySelector<HTMLTextAreaElement>('textarea[name="value"]')!,
      "   ",
    );

    document.querySelector<HTMLFormElement>("form")?.requestSubmit();
    await nextTick();

    expect(controller.setSecret).toHaveBeenCalledWith("SPACES", "   ");
  });
});

describe("SecretsPanel history", () => {
  it("opens the history of a row from its menu", async () => {
    controller.secrets.value = [API_KEY] as never;
    render();
    await nextTick();
    expect(history.key).toBeNull();

    const menu = await openRowMenu("API_KEY");
    Array.from(menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'))
      .find((item) => item.textContent?.trim() === "History")!
      .click();
    await nextTick();

    expect(history.key).toBe("API_KEY");
  });
});

describe("SecretsPanel table", () => {
  it("lists the row actions, delete last", async () => {
    controller.secrets.value = [API_KEY] as never;
    render();
    await nextTick();

    const menu = await openRowMenu("API_KEY");

    expect(
      Array.from(menu.querySelectorAll('[role="menuitem"]')).map((item) =>
        item.textContent?.trim(),
      ),
    ).toEqual(["Reveal", "Copy value", "History", "Edit value", "Delete"]);
  });

  it("copies a value from the row menu", async () => {
    controller.secrets.value = [API_KEY] as never;
    render();
    await nextTick();

    const menu = await openRowMenu("API_KEY");
    Array.from(menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'))
      .find((item) => item.textContent?.trim() === "Copy value")!
      .click();

    expect(controller.copyValue).toHaveBeenCalledWith("API_KEY");
  });

  it("reveals a value by clicking its mask", async () => {
    controller.secrets.value = [API_KEY] as never;
    render();
    await nextTick();

    document
      .querySelector<HTMLButtonElement>('button[aria-label="Reveal API_KEY"]')!
      .click();

    expect(controller.reveal).toHaveBeenCalledWith("API_KEY");
  });

  it("offers only what a reader can do", async () => {
    controller.secrets.value = [API_KEY] as never;
    render("read");
    await nextTick();

    expect(button("Add secret")).toBeUndefined();
    expect(button("Import")).toBeUndefined();
    const menu = await openRowMenu("API_KEY");
    expect(
      Array.from(menu.querySelectorAll('[role="menuitem"]')).map((item) =>
        item.textContent?.trim(),
      ),
    ).toEqual(["Reveal", "Copy value", "History"]);
  });

  it("says when no key matches the filter", async () => {
    controller.secrets.value = [API_KEY] as never;
    controller.filter.value = "NOPE";
    render();
    await nextTick();

    expect(
      document.querySelector('[data-testid="secrets-no-match"]')?.textContent,
    ).toContain("No keys match “NOPE”.");
  });
});

describe("SecretsPanel empty environment", () => {
  it("walks a writer through the first secrets", async () => {
    render();
    await nextTick();

    const checklist = document.querySelector('[data-testid="first-run"]');
    expect(checklist?.textContent).toContain("Get payments running");
    expect(checklist?.textContent).toContain(
      "payments, with dev and production.",
    );
    expect(
      checklist?.querySelector('[aria-current="step"]')?.textContent,
    ).toContain("Add secrets to");
    expect(checklist?.textContent).toContain(
      "dopbase import payments/production .env",
    );
    expect(checklist?.textContent).toContain(
      "dopbase run payments/production -- npm run dev",
    );
    const dropZone = checklist?.querySelector('[data-testid="env-drop-zone"]');
    expect(dropZone?.textContent).toContain("Choose a file");
    expect(dropZone?.textContent).toContain("Paste .env");
    expect(dropZone?.textContent).toContain("Add secret");
    expect(
      document.querySelector('[data-testid="secrets-toolbar"]'),
    ).toBeNull();
  });

  it("waits for the project's environments before picking a start", async () => {
    empty.view.value = "loading";
    render();
    await nextTick();

    expect(
      document.querySelector('[data-testid="empty-skeleton"]'),
    ).not.toBeNull();
    expect(document.querySelector('[data-testid="first-run"]')).toBeNull();
  });

  it("starts from another environment's keys, values left empty by default", async () => {
    empty.view.value = "start";
    empty.sources.value = [
      { id: "env_dev", name: "dev", secretCount: 14, canCopyValues: true },
      { id: "env_qa", name: "qa", secretCount: 2, canCopyValues: true },
    ];
    render();
    await nextTick();

    const start = document.querySelector('[data-testid="empty-start"]');
    expect(start?.textContent).toContain("production has no secrets yet");
    expect(start?.textContent).toContain("dev already has 14.");
    expect(start?.textContent).toContain("Values start empty");
    expect(
      start?.querySelector('[data-testid="env-drop-zone"]'),
    ).not.toBeNull();
    expect(start?.textContent).toContain(
      "dopbase import payments/production .env",
    );
    expect(document.querySelector('[data-testid="first-run"]')).toBeNull();

    button("Review 14 keys")?.click();
    await nextTick();
    expect(copyDialog).toMatchObject({
      open: true,
      source: { name: "dev" },
      withValues: false,
    });
  });

  it("asks for values only when chosen and readable", async () => {
    empty.view.value = "start";
    empty.sources.value = [
      { id: "env_dev", name: "dev", secretCount: 14, canCopyValues: true },
    ];
    render();
    await nextTick();

    button("Keys and values")?.click();
    await nextTick();
    button("Review 14 keys")?.click();
    await nextTick();

    expect(copyDialog.withValues).toBe(true);
  });

  it("copies only keys from an environment you can't read", async () => {
    empty.view.value = "start";
    empty.sources.value = [
      { id: "env_prod", name: "prod", secretCount: 1, canCopyValues: false },
    ];
    render();
    await nextTick();

    expect(button("Keys and values")).toBeUndefined();
    expect(document.body.textContent).toContain("but not its values");
    expect(button("Review 1 key")).toBeDefined();
  });

  it("tells a reader why they can't add secrets and where to ask", async () => {
    render("read", true);
    await nextTick();

    expect(document.querySelector('[data-testid="first-run"]')).toBeNull();
    const state = document.querySelector(
      '[data-testid="secrets-read-only-empty"]',
    );
    expect(state?.textContent).toContain("production has no secrets yet");
    expect(state?.textContent).toContain("adding secrets needs write access");
    expect(state?.textContent).toContain("dopbase run payments/production");

    button("See who has access")?.click();
    expect(routerPush).toHaveBeenCalledWith({
      name: "project-access",
      params: { projectRef: "payments" },
    });
  });

  it("offers no Access link to a reader who can't open the Access tab", async () => {
    render("read");
    await nextTick();

    const state = document.querySelector(
      '[data-testid="secrets-read-only-empty"]',
    );
    expect(state?.textContent).toContain(
      "A project manager of payments can add them",
    );
    expect(button("See who has access")).toBeUndefined();
  });
});
