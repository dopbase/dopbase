<script setup lang="ts">
import { useSsoController } from "./Sso.controller";
import type { SsoStatus } from "./Sso.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCode,
  DbConfirmDialog,
  DbCopyButton,
  DbInput,
  DbSelect,
  DbSkeleton,
  DbSwitch,
} from "~/components/ui";
import {
  AlertTriangleIcon,
  InfoIcon,
  KeyIcon,
  RefreshIcon,
  TrashIcon,
} from "~/assets/icons";
import { formatDateTime } from "~/utils/format";

/**
 * Single sign-on: root configures one OpenID Connect provider, proves it
 * with a test sign-in, then turns it on for everyone.
 */
const {
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
  presetHint,
  presetOptions,
  blocked,
  saving,
  toggling,
  requiring,
  confirmingRequire,
  hidingForm,
  testing,
  notice,
  actionError,
  confirmingRemove,
  removing,
  removeError,
  choosePreset,
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
} = useSsoController();

const statusBadge: Record<
  SsoStatus,
  { label: string; tone: "ok" | "warn" | "neutral" }
> = {
  none: { label: "Not set up", tone: "neutral" },
  unverified: { label: "Untested", tone: "warn" },
  ready: { label: "Off", tone: "neutral" },
  enabled: { label: "On", tone: "ok" },
};

/** Left column of each Provider row: a short label and what it's for. */
const rowClass =
  "grid grid-cols-1 gap-x-8 gap-y-3 border-t border-line-soft py-4 md:grid-cols-[minmax(0,280px)_minmax(0,1fr)]";
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Single sign-on"
        subtitle="Let people sign in with your identity provider. Root can always use its password." />

      <DbSkeleton v-if="loading" class="h-64 rounded-card" />
      <DbAlert v-else-if="loadError">{{ loadError }}</DbAlert>

      <template v-else-if="settings">
        <!-- An insecure public URL blocks everything below. -->
        <section
          v-if="blocked"
          class="flex gap-3 rounded-card border border-warn/40 bg-warn/10 p-4 sm:p-5"
          data-testid="sso-blocked">
          <AlertTriangleIcon class="mt-0.5 h-5 w-5 shrink-0 text-warn" />
          <div class="flex min-w-0 flex-col gap-2 text-sm text-ink">
            <h2 class="font-semibold text-ink-strong">
              Single sign-on needs an HTTPS public URL
            </h2>
            <p>
              This server's public URL is
              <span
                class="font-mono text-ink-strong [overflow-wrap:anywhere]"
                >{{ settings.publicUrl }}</span
              >. Your provider would send sign-in codes back to it over plain
              HTTP, readable on the network, so setup stays locked until the URL
              is HTTPS. Restart the server with it:
            </p>
            <DbCode class="truncate"
              >dopbase server start --public-url
              https://dopbase.example.com</DbCode
            >
            <p class="text-ink-muted">
              You can also set
              <span class="font-mono">DOPBASE_PUBLIC_URL</span> or
              <span class="font-mono">public_url</span> in server.toml.
              <span class="font-mono">http://localhost</span> works for trying
              it out on one machine.
            </p>
          </div>
        </section>

        <DbAlert v-if="notice" tone="success">{{ notice }}</DbAlert>
        <DbAlert v-if="actionError">{{ actionError }}</DbAlert>

        <fieldset
          :disabled="blocked"
          class="flex min-w-0 flex-col gap-5"
          :class="blocked ? 'pointer-events-none opacity-50' : ''">
          <!-- Where setup stands, with the test and on/off buttons -->
          <section
            class="flex flex-col gap-3.5 rounded-card border border-line bg-panel p-4 sm:p-5"
            data-testid="sso-summary">
            <div
              class="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-3.5">
              <div class="flex min-w-0 flex-1 items-center gap-3.5">
                <div
                  class="flex h-10 w-10 shrink-0 items-center justify-center rounded-control bg-raised text-[15px] font-semibold text-ink-strong"
                  aria-hidden="true">
                  <template v-if="provider">{{
                    providerName.charAt(0).toUpperCase()
                  }}</template>
                  <KeyIcon v-else class="h-4 w-4 text-ink-muted" />
                </div>
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <h2
                      class="text-[15px] font-semibold text-ink-strong [overflow-wrap:anywhere]">
                      {{ provider ? providerName : "No provider yet" }}
                    </h2>
                    <DbBadge
                      :tone="statusBadge[status].tone"
                      data-testid="sso-status">
                      {{ statusBadge[status].label }}
                    </DbBadge>
                  </div>
                  <p
                    class="text-sm text-ink-muted"
                    data-testid="sso-summary-line">
                    {{ providerSummary }}
                    <template v-if="provider?.verifiedAt">
                      Last test sign-in
                      {{ formatDateTime(provider.verifiedAt) }}.
                    </template>
                  </p>
                </div>
              </div>
              <div v-if="provider" class="flex shrink-0 flex-wrap gap-2">
                <DbButton
                  size="sm"
                  :variant="status === 'unverified' ? 'primary' : 'secondary'"
                  :loading="testing"
                  :disabled="blocked"
                  data-testid="sso-test"
                  @click="testSignIn">
                  <RefreshIcon v-if="!testing" class="h-3.5 w-3.5" />
                  Test sign-in
                </DbButton>
                <DbButton
                  size="sm"
                  :variant="status === 'ready' ? 'primary' : 'secondary'"
                  :disabled="status === 'unverified' || blocked"
                  :loading="toggling"
                  data-testid="sso-toggle"
                  @click="toggle">
                  {{ provider.enabled ? "Turn off" : "Turn on" }}
                </DbButton>
              </div>
            </div>
            <div
              v-if="provider && provider.passwordlessAccounts > 0"
              class="flex flex-col gap-2 rounded-control bg-warn/10 py-2.5 pl-3.5 pr-2.5 text-sm text-ink sm:flex-row sm:items-center"
              data-testid="sso-passwordless-warning">
              <div class="flex min-w-0 flex-1 gap-2.5">
                <AlertTriangleIcon class="mt-0.5 h-4 w-4 shrink-0 text-warn" />
                <p>
                  {{ provider.passwordlessAccounts }}
                  {{
                    provider.passwordlessAccounts === 1
                      ? "account has"
                      : "accounts have"
                  }}
                  no password. Turning single sign-on off or removing it locks
                  {{ provider.passwordlessAccounts === 1 ? "it" : "them" }} out
                  until you set one.
                </p>
              </div>
              <RouterLink
                to="/users"
                class="inline-flex h-8 shrink-0 items-center self-start rounded-control px-3 text-xs font-semibold text-ink hover:bg-raised hover:text-ink-strong sm:self-auto">
                Open Users
              </RouterLink>
            </div>
          </section>

          <!-- Provider details -->
          <form
            class="flex flex-col rounded-card border border-line bg-panel p-4 sm:p-5"
            novalidate
            data-testid="sso-form"
            @submit.prevent="save">
            <div class="pb-4">
              <h2 class="text-[15px] font-semibold text-ink-strong">
                Provider
              </h2>
              <p class="text-sm text-ink-muted">
                The OpenID Connect app people sign in with.
              </p>
            </div>

            <div :class="rowClass">
              <div class="flex flex-col gap-1 text-sm">
                <h3 class="font-semibold text-ink-strong">Redirect URI</h3>
                <p class="text-ink-muted">
                  Add this to the web app you create at your provider. The app
                  uses the authorization code flow and has a client secret.
                </p>
              </div>
              <div class="flex min-w-0 items-center gap-2 self-start">
                <DbCode class="min-w-0 flex-1 truncate h-8 pt-1.5">{{
                  settings.redirectUri
                }}</DbCode>
                <DbCopyButton :value="settings.redirectUri" />
              </div>
            </div>

            <div :class="rowClass">
              <div class="flex flex-col gap-1 text-sm">
                <h3 class="font-semibold text-ink-strong">Connection</h3>
                <p class="text-ink-muted">
                  Dopbase checks the issuer when you save. A first sign-in
                  matches an existing account by verified email, so you still
                  create accounts on the Users page.
                </p>
              </div>
              <div class="flex min-w-0 flex-col gap-3">
                <div class="flex flex-col gap-1.5">
                  <DbSelect
                    :model-value="preset"
                    label="Provider"
                    :options="presetOptions"
                    :disabled="blocked"
                    @update:model-value="choosePreset" />
                  <p
                    class="text-xs text-ink-muted"
                    data-testid="sso-preset-hint">
                    {{ presetHint }}
                  </p>
                </div>
                <DbInput
                  v-model="issuer"
                  label="Issuer URL"
                  name="issuer"
                  mono
                  :placeholder="issuerPlaceholder"
                  :disabled="blocked"
                  :error="fieldErrors.issuer" />
                <DbInput
                  v-model="clientId"
                  label="Client ID"
                  name="clientId"
                  mono
                  autocomplete="off"
                  placeholder="OICD Client ID"
                  :disabled="blocked"
                  :error="fieldErrors.clientId" />
                <DbInput
                  v-model="clientSecret"
                  label="Client secret"
                  name="clientSecret"
                  type="password"
                  autocomplete="new-password"
                  :hint="
                    provider
                      ? 'Stored encrypted. Leave empty to keep it.'
                      : null
                  "
                  :disabled="blocked"
                  :error="fieldErrors.clientSecret" />
              </div>
            </div>

            <div :class="rowClass">
              <div class="flex flex-col gap-1 text-sm">
                <h3 class="font-semibold text-ink-strong">Sign-in button</h3>
                <p class="text-ink-muted">
                  Shown on the sign-in page, after “Sign in with”.
                </p>
              </div>
              <DbInput
                v-model="displayName"
                label="Button label"
                name="displayName"
                :disabled="blocked"
                :error="fieldErrors.displayName" />
            </div>

            <div
              class="flex flex-col gap-3 border-t border-line-soft pt-4 sm:flex-row sm:items-center sm:gap-4">
              <p
                class="flex min-w-0 flex-1 gap-2 text-sm text-ink-muted"
                data-testid="sso-save-note">
                <InfoIcon class="mt-0.5 h-4 w-4 shrink-0 text-ink-faint" />
                <span v-if="provider">
                  A new issuer, client ID or secret turns single sign-on off
                  until the next test sign-in. A new issuer also removes every
                  linked identity, and people link again by verified email.
                </span>
                <span v-else>
                  Saving checks the issuer. Run a test sign-in from the card
                  above next.
                </span>
              </p>
              <DbButton
                variant="primary"
                type="submit"
                class="self-start sm:self-auto"
                :loading="saving"
                :disabled="blocked">
                {{ provider ? "Save changes" : "Save" }}
              </DbButton>
            </div>
          </form>

          <!-- Policy, once single sign-on is on -->
          <section
            v-if="provider?.enabled"
            class="flex flex-col rounded-card border border-line bg-panel p-4 sm:p-5"
            data-testid="sso-options">
            <div class="pb-4">
              <h2 class="text-[15px] font-semibold text-ink-strong">Policy</h2>
              <p class="text-sm text-ink-muted">
                Applies while single sign-on is on.
              </p>
            </div>
            <div
              class="flex items-center gap-4 border-t border-line-soft py-3.5">
              <div class="flex min-w-0 flex-1 flex-col gap-0.5 text-sm">
                <p class="font-semibold text-ink-strong">
                  Hide the password form
                </p>
                <p class="text-ink-muted">
                  The sign-in page shows only the
                  {{ provider.displayName }} button. Passwords still work in the
                  CLI and the API, and root can reach the form at
                  <span class="font-mono">/login?password</span>.
                </p>
              </div>
              <div class="flex shrink-0 items-center gap-2">
                <DbSwitch
                  :model-value="provider.hidePasswordForm"
                  label="Hide the password form"
                  hide-label
                  :disabled="hidingForm"
                  data-testid="sso-hide-form"
                  @update:model-value="setPasswordFormHidden" />
                <span
                  class="hidden w-6 text-sm text-ink sm:inline"
                  aria-hidden="true">
                  {{ provider.hidePasswordForm ? "On" : "Off" }}
                </span>
              </div>
            </div>
            <div
              class="flex items-center gap-4 border-t border-line-soft pt-3.5">
              <div class="flex min-w-0 flex-1 flex-col gap-0.5 text-sm">
                <p class="font-semibold text-ink-strong">
                  Require single sign-on
                </p>
                <p class="text-ink-muted">
                  Everyone except root must use {{ provider.displayName }}, in
                  the browser and the CLI. Their password sessions end. Root
                  keeps its password in case the provider is down.
                </p>
              </div>
              <div class="flex shrink-0 items-center gap-2">
                <DbSwitch
                  :model-value="provider.requireSso"
                  label="Require single sign-on"
                  hide-label
                  :disabled="requiring"
                  data-testid="sso-require"
                  @update:model-value="changeRequired" />
                <span
                  class="hidden w-6 text-sm text-ink sm:inline"
                  aria-hidden="true">
                  {{ provider.requireSso ? "On" : "Off" }}
                </span>
              </div>
            </div>
          </section>

          <!-- Danger zone -->
          <section
            v-if="provider"
            class="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-card border border-crit/60 bg-panel p-4 sm:p-5"
            data-testid="sso-remove">
            <div class="min-w-0 flex-1 basis-72">
              <h2 class="text-[15px] font-semibold text-crit">
                Remove provider
              </h2>
              <p class="text-sm text-ink-muted">{{ removeSummary }}</p>
            </div>
            <DbButton size="sm" :disabled="blocked" @click="promptRemove">
              <TrashIcon class="h-3.5 w-3.5" />
              Remove provider
            </DbButton>
          </section>
        </fieldset>
      </template>
    </div>

    <DbConfirmDialog
      :open="confirmingRequire"
      title="Require single sign-on?"
      :description="`Everyone except root is signed out now and signs in again with ${provider?.displayName ?? 'single sign-on'}. Password sign-in stops working for them in the browser and the CLI.`"
      confirm-label="Require single sign-on"
      tone="primary"
      :loading="requiring"
      @confirm="toggleRequired"
      @close="closeRequire" />

    <DbConfirmDialog
      :open="confirmingRemove"
      title="Remove single sign-on?"
      description="Every linked identity is removed. People keep their passwords and sign in with them."
      confirm-label="Remove provider"
      tone="danger"
      :affected-counts="
        provider
          ? [{ label: 'linked accounts', count: provider.linkedAccounts }]
          : undefined
      "
      :loading="removing"
      :error="removeError"
      @confirm="confirmRemove"
      @close="closeRemove" />
  </DashboardLayout>
</template>
