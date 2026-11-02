import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useQuery, useQueryCache } from "@pinia/colada";
import { useReauthentication } from "~/composable";
import { startSsoLink } from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import {
  deleteSsoProvider,
  saveSsoProvider,
  setSsoEnabled,
  setSsoPasswordFormHidden,
  setSsoRequired,
} from "~/services/sso.api";
import { GENERIC_PRESET } from "~/services/sso.api";
import type { SsoPreset, SsoSettings } from "~/services/sso.api";
import { loadFailure } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { ssoSettingsQuery } from "~/queries/sso";
import { ssoErrorMessage } from "~/utils/sso";

export type SsoStatus = "none" | "unverified" | "ready" | "enabled";

type FieldErrors = {
  displayName?: string;
  issuer?: string;
  clientId?: string;
  clientSecret?: string;
};

function cancelled(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

/**
 * Single sign-on settings, root only. Saving checks the provider's
 * discovery document; a test sign-in links root's own identity and must
 * succeed before single sign-on can be turned on.
 */
export function useSsoController() {
  const route = useRoute();
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const cache = useQueryCache();
  const invalidate = useInvalidate();
  const settingsQuery = useQuery(ssoSettingsQuery);
  const settings = computed<SsoSettings | null>(
    () => settingsQuery.data.value ?? null,
  );
  /** True until the first answer, good or bad. */
  const loading = settingsQuery.isPending;
  const loadError = loadFailure(
    settingsQuery,
    "Could not load the single sign-on settings.",
  );

  const preset = ref(GENERIC_PRESET);
  const displayName = ref("");
  const issuer = ref("");
  const clientId = ref("");
  const clientSecret = ref("");
  const fieldErrors = ref<FieldErrors>({});

  const saving = ref(false);
  const toggling = ref(false);
  const requiring = ref(false);
  const confirmingRequire = ref(false);
  const hidingForm = ref(false);
  const testing = ref(false);
  const notice = ref<string | null>(null);
  const actionError = ref<string | null>(null);

  const confirmingRemove = ref(false);
  const removing = ref(false);
  const removeError = ref<string | null>(null);

  const provider = computed(() => settings.value?.provider ?? null);
  const status = computed<SsoStatus>(() => {
    if (!provider.value) return "none";
    if (provider.value.enabled) return "enabled";
    return provider.value.verifiedAt ? "ready" : "unverified";
  });
  /** The server's preset list, which comes from its presets.toml. */
  const presets = computed<SsoPreset[]>(() => settings.value?.presets ?? []);
  const presetOptions = computed(() =>
    presets.value.map((entry) => ({ value: entry.id, label: entry.label })),
  );

  /** A stored ID no longer in the list behaves like the generic preset. */
  function findPreset(id: string): SsoPreset | undefined {
    return (
      presets.value.find((entry) => entry.id === id) ??
      presets.value.find((entry) => entry.id === GENERIC_PRESET)
    );
  }

  const issuerPlaceholder = computed(
    () => findPreset(preset.value)?.issuer ?? "",
  );
  const presetHint = computed(() => findPreset(preset.value)?.hint ?? "");
  /** The summary card's title: the preset's name, or the button label for the generic one. */
  const providerName = computed(() => {
    const current = provider.value;
    if (!current) return "";
    const entry = presets.value.find((item) => item.id === current.preset);
    return entry && entry.id !== GENERIC_PRESET
      ? entry.label
      : current.displayName;
  });

  /** One line under the provider's name saying where setup stands. */
  const providerSummary = computed(() => {
    const current = provider.value;
    if (!current) {
      return "Fill in the provider below and save. Then run a test sign-in before you turn it on.";
    }
    if (status.value === "enabled") {
      const linked = current.linkedAccounts;
      return linked === 1
        ? "1 person has linked their account."
        : `${linked} people have linked their account.`;
    }
    if (status.value === "ready") {
      return "The last test sign-in worked, so you can turn it on.";
    }
    return "Saved. Run a test sign-in as root before you turn it on. Nobody else sees a change yet.";
  });

  /** What removing the provider costs, for the danger card. */
  const removeSummary = computed(() => {
    const linked = provider.value?.linkedAccounts ?? 0;
    const unlinks =
      linked === 0
        ? "No accounts are linked yet."
        : `Unlinks ${linked} ${linked === 1 ? "account" : "accounts"}.`;
    return `${unlinks} People without a password can't sign in until you set one.`;
  });

  /** Without an HTTPS public URL nothing here can be set up. */
  const blocked = computed(
    () => settings.value !== null && !settings.value.publicUrlSecure,
  );

  /** Puts the saved provider in the form. The client secret is never sent back. */
  function fillForm(next: SsoSettings): void {
    const current = next.provider;
    preset.value = current?.preset ?? GENERIC_PRESET;
    displayName.value = current?.displayName ?? "";
    issuer.value = current?.issuer ?? "";
    clientId.value = current?.clientId ?? "";
    clientSecret.value = "";
  }

  /**
   * Shows settings the server just returned: in the cache, in the form, and
   * on every screen that shows sign-in methods.
   */
  function fill(next: SsoSettings): void {
    cache.setQueryData(ssoSettingsQuery.key, next);
    fillForm(next);
    void invalidate.signInChanged(ssoSettingsQuery.key);
  }

  // The form fills once from the first answer, so a background refresh
  // never overwrites what is being typed.
  let formFilled = false;
  watch(
    settingsQuery.data,
    (next) => {
      if (!next || formFilled) return;
      formFilled = true;
      fillForm(next);
    },
    { immediate: true },
  );

  function choosePreset(value: string): void {
    const previousDefault = findPreset(preset.value)?.buttonLabel;
    preset.value = value;
    if (displayName.value === "" || displayName.value === previousDefault) {
      displayName.value = findPreset(value)?.buttonLabel ?? "";
    }
  }

  /** Turn the callback's query into a message once, then drop it. */
  function readCallback(): void {
    if (route.query.linked === "1") {
      notice.value = "Test sign-in worked. You can turn on single sign-on now.";
    }
    const error = ssoErrorMessage(route.query.sso_error);
    if (error) actionError.value = error;
    if (route.query.linked || route.query.sso_error) {
      void router.replace({ query: {} });
    }
  }

  /** Reads the settings again and puts them in the form. */
  async function load(): Promise<void> {
    await settingsQuery.refetch();
    const next = settingsQuery.data.value;
    if (next) fillForm(next);
  }

  function validate(): boolean {
    const errors: FieldErrors = {};
    if (displayName.value.trim() === "") {
      errors.displayName = "Enter the label for the sign-in button.";
    }
    if (issuer.value.trim() === "") {
      errors.issuer = "Enter the issuer URL.";
    }
    if (clientId.value.trim() === "") {
      errors.clientId = "Enter the client ID.";
    }
    if (!provider.value && clientSecret.value.trim() === "") {
      errors.clientSecret = "Enter the client secret.";
    }
    fieldErrors.value = errors;
    return Object.keys(errors).length === 0;
  }

  function mapSaveError(error: unknown): void {
    if (!(error instanceof ApiError)) {
      actionError.value = "The settings could not be saved.";
      return;
    }
    const errors: FieldErrors = {};
    if (error.hasCode("SSO_NAME_INVALID")) {
      errors.displayName = error.codes.SSO_NAME_INVALID;
    }
    if (error.hasCode("SSO_ISSUER_INVALID")) {
      errors.issuer = error.codes.SSO_ISSUER_INVALID;
    }
    if (error.hasCode("SSO_DISCOVERY_FAILED")) {
      errors.issuer = error.codes.SSO_DISCOVERY_FAILED;
    }
    if (error.hasCode("SSO_CLIENT_INVALID")) {
      const message = error.codes.SSO_CLIENT_INVALID ?? "";
      if (message.toLowerCase().includes("secret")) {
        errors.clientSecret = message;
      } else {
        errors.clientId = message;
      }
    }
    fieldErrors.value = errors;
    if (Object.keys(errors).length === 0) actionError.value = error.message;
  }

  async function save(): Promise<void> {
    notice.value = null;
    actionError.value = null;
    if (!validate()) return;
    const hadProvider = provider.value !== null;
    saving.value = true;
    try {
      await runWithReauth(async () => {
        const secret = clientSecret.value.trim();
        fill(
          await saveSsoProvider({
            preset: preset.value,
            displayName: displayName.value.trim(),
            issuer: issuer.value.trim(),
            clientId: clientId.value.trim(),
            ...(secret ? { clientSecret: secret } : {}),
          }),
        );
      });
      notice.value =
        hadProvider && status.value !== "unverified"
          ? "Settings saved."
          : "Settings saved. Run a test sign-in next.";
    } catch (error) {
      if (!cancelled(error)) mapSaveError(error);
    } finally {
      saving.value = false;
    }
  }

  async function testSignIn(): Promise<void> {
    actionError.value = null;
    testing.value = true;
    try {
      window.location.assign(await startSsoLink());
    } catch (error) {
      actionError.value =
        error instanceof ApiError
          ? error.message
          : "Could not start the test sign-in.";
      testing.value = false;
    }
  }

  async function toggle(): Promise<void> {
    if (!provider.value) return;
    notice.value = null;
    actionError.value = null;
    const enable = !provider.value.enabled;
    toggling.value = true;
    try {
      await runWithReauth(async () => {
        fill(await setSsoEnabled(enable));
      });
      notice.value = enable
        ? "Single sign-on is on. The sign-in page now shows the button."
        : "Single sign-on is off. Everyone signs in with a password.";
    } catch (error) {
      if (!cancelled(error)) {
        actionError.value =
          error instanceof ApiError
            ? error.message
            : "The setting could not be changed.";
      }
    } finally {
      toggling.value = false;
    }
  }

  /** Turning the requirement on signs people out, so it asks first. */
  function changeRequired(required: boolean): void {
    if (required) confirmingRequire.value = true;
    else void toggleRequired();
  }

  function closeRequire(): void {
    if (!requiring.value) confirmingRequire.value = false;
  }

  async function setPasswordFormHidden(hidden: boolean): Promise<void> {
    notice.value = null;
    actionError.value = null;
    hidingForm.value = true;
    try {
      await runWithReauth(async () => {
        fill(await setSsoPasswordFormHidden(hidden));
      });
      notice.value = hidden
        ? "The sign-in page now shows only the single sign-on button."
        : "The sign-in page shows the password form again.";
    } catch (error) {
      if (!cancelled(error)) {
        actionError.value =
          error instanceof ApiError
            ? error.message
            : "The setting could not be changed.";
      }
    } finally {
      hidingForm.value = false;
    }
  }

  async function toggleRequired(): Promise<void> {
    if (!provider.value) return;
    notice.value = null;
    actionError.value = null;
    const required = !provider.value.requireSso;
    requiring.value = true;
    try {
      await runWithReauth(async () => {
        fill(await setSsoRequired(required));
      });
      confirmingRequire.value = false;
      notice.value = required
        ? "Single sign-on is now required. Everyone except root was signed out and signs in again with single sign-on."
        : "Passwords work again for everyone.";
    } catch (error) {
      if (!cancelled(error)) {
        actionError.value =
          error instanceof ApiError
            ? error.message
            : "The setting could not be changed.";
      }
    } finally {
      requiring.value = false;
    }
  }

  function promptRemove(): void {
    removeError.value = null;
    confirmingRemove.value = true;
  }

  function closeRemove(): void {
    if (removing.value) return;
    confirmingRemove.value = false;
  }

  async function confirmRemove(): Promise<void> {
    removing.value = true;
    removeError.value = null;
    try {
      await runWithReauth(deleteSsoProvider);
      confirmingRemove.value = false;
      notice.value = "Single sign-on was removed.";
      await load();
      void invalidate.signInChanged(ssoSettingsQuery.key);
    } catch (error) {
      if (!cancelled(error)) {
        removeError.value =
          error instanceof ApiError
            ? error.message
            : "Single sign-on could not be removed.";
      }
    } finally {
      removing.value = false;
    }
  }

  onMounted(readCallback);

  return {
    settings,
    provider,
    providerName,
    providerSummary,
    removeSummary,
    status,
    loading,
    loadError,
    preset,
    displayName,
    issuer,
    clientId,
    clientSecret,
    fieldErrors,
    issuerPlaceholder,
    presetOptions,
    saving,
    toggling,
    requiring,
    confirmingRequire,
    hidingForm,
    presetHint,
    blocked,
    testing,
    notice,
    actionError,
    confirmingRemove,
    removing,
    removeError,
    choosePreset,
    load,
    save,
    testSignIn,
    toggle,
    toggleRequired,
    changeRequired,
    closeRequire,
    setPasswordFormHidden,
    promptRemove,
    closeRemove,
    confirmRemove,
  };
}
