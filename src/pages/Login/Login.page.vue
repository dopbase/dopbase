<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useLoginController } from "./Login.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbInput, DbSkeleton } from "~/components/ui";
import { ShieldIcon } from "~/assets/icons";

const {
  email,
  password,
  fieldErrors,
  formError,
  submitting,
  submit,
  ssoName,
  methodsLoaded,
  showPasswordForm,
  ssoHref,
  ssoError,
  loadMethods,
  host,
  version,
  capsLock,
  trackCapsLock,
} = useLoginController();
onMounted(loadMethods);
const route = useRoute();
const notice = computed(() => {
  if (route.query.notice === "password-changed") {
    return "Your password was changed. Sign in with the new one.";
  }
  if (route.query.notice === "backup-restored") {
    return "Backup restored. Sign in with an account from the backup.";
  }
  if (route.query.notice === "cli-approve") {
    return "Sign in to approve the request from your terminal. You'll go straight back to it.";
  }
  return null;
});
/** Passwords are off and the provider is the only way in. */
const ssoOnly = computed(
  () => ssoName.value !== null && !showPasswordForm.value,
);
</script>

<template>
  <AuthLayout>
    <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
      Sign in
    </h1>
    <p class="mt-2 text-sm text-ink-muted">
      to <span class="font-mono text-ink">{{ host }}</span>
    </p>

    <DbAlert v-if="notice" tone="info" class="mt-6">{{ notice }}</DbAlert>
    <DbAlert v-if="ssoError" class="mt-6">{{ ssoError }}</DbAlert>

    <!-- Wait for the sign-in options so a hidden form never flashes. -->
    <DbSkeleton
      v-if="!methodsLoaded"
      class="mt-8 h-11 rounded-control"
      data-testid="login-loading" />

    <!-- Passwords are turned off: the provider is the way in. -->
    <div
      v-else-if="ssoOnly"
      class="mt-8 flex flex-col gap-4"
      data-testid="sso-only">
      <div
        class="flex items-start gap-3 rounded-card border border-line bg-panel px-4 py-3">
        <ShieldIcon class="mt-0.5 h-4 w-4 shrink-0 text-ink-muted" />
        <div class="text-sm">
          <p class="font-medium text-ink-strong">
            {{ host }} uses {{ ssoName }} sign-in
          </p>
          <p class="text-ink-muted">
            Passwords are turned off for this server.
          </p>
        </div>
      </div>
      <a
        :href="ssoHref"
        data-testid="sso-login"
        class="inline-flex h-10 items-center justify-center rounded-control bg-accent px-4 text-sm font-semibold text-white transition-colors hover:bg-accent-alt1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
        Continue with {{ ssoName }}
      </a>
      <RouterLink
        :to="{ name: 'login', query: { ...route.query, password: null } }"
        class="self-start text-sm text-ink-muted underline underline-offset-4 hover:text-ink">
        Signing in as root? Use your password
      </RouterLink>
    </div>

    <template v-else>
      <div v-if="ssoName" class="mt-8 flex flex-col gap-4">
        <a
          :href="ssoHref"
          data-testid="sso-login"
          class="inline-flex h-10 items-center justify-center rounded-control border border-line bg-raised px-4 text-sm font-medium text-ink-strong transition-colors hover:bg-line focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent">
          Continue with {{ ssoName }}
        </a>
        <p
          class="flex items-center gap-3 text-xs text-ink-faint before:h-px before:flex-1 before:bg-line after:h-px after:flex-1 after:bg-line">
          or with email
        </p>
      </div>

      <form
        :class="ssoName ? 'mt-4' : 'mt-8'"
        class="flex flex-col gap-3"
        novalidate
        data-testid="login-form"
        @submit.prevent="submit">
        <DbInput
          v-model="email"
          label="Email"
          name="email"
          type="email"
          autocomplete="email"
          placeholder="you@example.com"
          :error="fieldErrors.email" />

        <div class="flex flex-col gap-1">
          <DbInput
            v-model="password"
            label="Password"
            name="password"
            type="password"
            autocomplete="current-password"
            :error="fieldErrors.password"
            @keydown="trackCapsLock"
            @keyup="trackCapsLock"
            @focusout="capsLock = false" />
          <p
            v-if="capsLock"
            class="text-[13px] text-warn"
            data-testid="caps-lock">
            Caps Lock is on
          </p>
        </div>

        <div class="flex justify-end">
          <RouterLink
            :to="{ name: 'login-help' }"
            class="text-[13px] text-ink-muted underline underline-offset-4 hover:text-ink">
            Forgot password?
          </RouterLink>
        </div>

        <p
          v-if="formError"
          class="text-sm text-crit"
          role="alert"
          data-testid="login-error">
          {{ formError }}
        </p>

        <DbButton variant="primary" type="submit" :loading="submitting">
          Sign in
        </DbButton>
      </form>
    </template>

    <template #footer>
      <RouterLink
        :to="{ name: 'login-help' }"
        class="underline underline-offset-4 hover:text-ink">
        Trouble signing in?
      </RouterLink>
      <span v-if="version">Self-hosted, Dopbase {{ version }}</span>
    </template>
  </AuthLayout>
</template>
