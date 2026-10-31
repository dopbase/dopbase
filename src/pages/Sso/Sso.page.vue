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
import { AlertTriangleIcon } from "~/assets/icons";
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
  none: { label: "not set up", tone: "neutral" },
  unverified: { label: "untested", tone: "warn" },
  ready: { label: "off", tone: "neutral" },
  enabled: { label: "on", tone: "ok" },
};
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
            v-if="provider"
            class="flex flex-col gap-3 rounded-card border border-line bg-panel p-4 sm:p-5"
            data-testid="sso-summary">
            <div
              class="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-3">
              <div class="flex min-w-0 flex-1 items-start gap-3">
                <DbBadge
                  class="mt-0.5 shrink-0"
                  :tone="statusBadge[status].tone"
                  data-testid="sso-status">
                  {{ statusBadge[status].label }}
                </DbBadge>
                <div class="min-w-0">
                  <h2
                    class="text-[15px] font-semibold text-ink-strong [overflow-wrap:anywhere]">
                    {{ providerName }}
                  </h2>
                  <p class="text-sm text-ink-muted">{{ providerSummary }}</p>
                  <p v-if="provider.verifiedAt" class="text-xs text-ink-muted">
                    Last test sign-in {{ formatDateTime(provider.verifiedAt) }}.
                  </p>
                </div>
              </div>
              <div class="flex shrink-0 flex-wrap gap-2">
                <DbButton
                  :variant="status === 'unverified' ? 'primary' : 'secondary'"
                  :loading="testing"
                  :disabled="blocked"
                  data-testid="sso-test"
                  @click="testSignIn">
                  Test sign-in
                </DbButton>
                <DbButton
                  :variant="
                    status === 'ready'
                      ? 'primary'
                      : provider.enabled
                        ? 'ghost'
                        : 'secondary'
                  "
                  :disabled="status === 'unverified' || blocked"
                  :loading="toggling"
                  data-testid="sso-toggle"
                  @click="toggle">
                  {{ provider.enabled ? "Turn off" : "Turn on" }}
                </DbButton>
              </div>
            </div>
            <div
              v-if="provider.passwordlessAccounts > 0"
              class="flex gap-2.5 rounded-control border border-warn/30 bg-warn/10 px-3.5 py-3 text-sm text-ink"
              data-testid="sso-passwordless-warning">
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
                until an administrator sets a password on the Users page.
              </p>
            </div>
          </section>

          <!-- Provider details -->
          <form
            class="flex flex-col gap-4 rounded-card border border-line bg-panel p-4 sm:p-5"
            novalidate
            data-testid="sso-form"
            @submit.prevent="save">
            <div class="flex items-center gap-3">
              <h2 class="flex-1 text-[15px] font-semibold text-ink-strong">
                Provider
              </h2>
              <DbButton
                variant="primary"
                type="submit"
                :loading="saving"
                :disabled="blocked">
                Save
              </DbButton>
            </div>

            <div class="flex flex-col gap-1.5">
              <p class="text-sm text-ink-muted">
                At your provider, create a web app that uses the authorization
                code flow and has a client secret, then add this redirect URI.
              </p>
              <div class="flex items-center gap-2">
                <DbCode class="min-w-0 flex-1 truncate">{{
                  settings.redirectUri
                }}</DbCode>
                <DbCopyButton :value="settings.redirectUri" />
              </div>
            </div>

            <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div class="flex flex-col gap-1.5">
                <DbSelect
                  :model-value="preset"
                  label="Provider"
                  :options="presetOptions"
                  :disabled="blocked"
                  @update:model-value="choosePreset" />
                <p class="text-xs text-ink-muted" data-testid="sso-preset-hint">
                  {{ presetHint }}
                </p>
              </div>
              <DbInput
                v-model="displayName"
                label="Button label"
                name="displayName"
                hint="Shown on the sign-in page button, after Sign in with."
                :disabled="blocked"
                :error="fieldErrors.displayName" />
            </div>
            <DbInput
              v-model="issuer"
              label="Issuer URL"
              name="issuer"
              mono
              :placeholder="issuerPlaceholder"
              :disabled="blocked"
              :error="fieldErrors.issuer" />
            <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <DbInput
                v-model="clientId"
                label="Client ID"
                name="clientId"
                mono
                autocomplete="off"
                :disabled="blocked"
                :error="fieldErrors.clientId" />
              <DbInput
                v-model="clientSecret"
                label="Client secret"
                name="clientSecret"
                type="password"
                autocomplete="new-password"
                :hint="
                  provider ? 'Stored encrypted. Leave empty to keep it.' : null
                "
                :disabled="blocked"
                :error="fieldErrors.clientSecret" />
            </div>
            <p class="text-xs text-ink-muted">
              You still create accounts on the Users page. A first sign-in
              matches an account by its verified email.
              <template v-if="provider">
                Changing the issuer, client ID or secret turns single sign-on
                off until the next test sign-in. A new issuer also removes every
                linked identity, and people link again by verified email when
                they next sign in.
              </template>
            </p>
          </form>

          <!-- Policy, once single sign-on is on -->
          <section
            v-if="provider?.enabled"
            class="flex flex-col gap-4 rounded-card border border-line bg-panel p-4 sm:p-5"
            data-testid="sso-options">
            <h2 class="text-[15px] font-semibold text-ink-strong">Policy</h2>
            <div class="flex items-start gap-3">
              <div class="flex min-w-0 flex-1 flex-col gap-0.5 text-sm">
                <p class="font-semibold text-ink-strong">
                  Hide the password form
                </p>
                <p class="text-ink-muted">
                  The sign-in page shows only the
                  {{ provider.displayName }} button. Passwords keep working in
                  the CLI and the API, and root can still reach the form at
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
            <div class="flex items-start gap-3">
              <div class="flex min-w-0 flex-1 flex-col gap-0.5 text-sm">
                <p class="font-semibold text-ink-strong">
                  Require single sign-on
                </p>
                <p class="text-ink-muted">
                  Everyone except root must use the provider, in the browser and
                  the CLI. Their password sessions end. Root keeps its password
                  in case the provider is down.
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
            class="flex flex-col gap-3 rounded-card border border-crit/60 bg-panel p-4 sm:flex-row sm:items-center sm:p-5"
            data-testid="sso-remove">
            <div class="min-w-0 flex-1">
              <h2 class="text-[15px] font-semibold text-crit">
                Remove provider
              </h2>
              <p class="text-sm text-ink-muted">{{ removeSummary }}</p>
            </div>
            <DbButton
              variant="danger"
              class="self-start sm:self-auto"
              :disabled="blocked"
              @click="promptRemove">
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
