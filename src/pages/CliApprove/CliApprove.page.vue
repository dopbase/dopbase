<script setup lang="ts">
import {
  CLOSE_DELAY_SECONDS,
  CODE_ALPHABET,
  CODE_LENGTH,
  displayCode,
  useCliApproveController,
} from "./CliApprove.controller";
import { FocusLayout } from "~/layouts";
import ReauthModal from "~/components/app/ReauthModal.vue";
import { DbAlert, DbButton, DbCodeInput, DbSpinner } from "~/components/ui";
import { CheckIcon, TerminalIcon, XIcon } from "~/assets/icons";
import { formatDateTime, formatRelativeTime } from "~/utils/format";

/**
 * CLI approval — where `dopbase login --browser` sends the person to
 * approve the code their terminal shows. A page of its own, with no Admin
 * UI navigation around it. It uses the browser's current session and keeps
 * the account it signs in on screen.
 */
const {
  step,
  code,
  request,
  account,
  isConfirmation,
  codeError,
  formError,
  looking,
  deciding,
  closingIn,
  closeBlocked,
  keepOpen,
  lookUp,
  approve,
  deny,
  switchAccount,
  startOver,
} = useCliApproveController();
</script>

<template>
  <FocusLayout>
    <!-- Checking a code that came in the link -->
    <div
      v-if="step === 'checking'"
      class="flex flex-col items-center gap-3 py-10 text-sm text-ink-muted"
      data-testid="cli-checking">
      <DbSpinner />
      {{ code.length === CODE_LENGTH ? "Checking the code…" : "One moment…" }}
    </div>

    <!-- Enter the code -->
    <form
      v-else-if="step === 'enter'"
      class="flex flex-col items-center gap-8 text-center"
      novalidate
      data-testid="cli-code-form"
      @submit.prevent="lookUp">
      <div
        class="flex h-12 w-12 items-center justify-center rounded-card border border-line bg-panel text-ink-muted">
        <TerminalIcon class="h-6 w-6" />
      </div>
      <div class="flex flex-col gap-2">
        <h1 class="text-2xl font-semibold text-ink-strong">
          Enter the code from your terminal
        </h1>
        <p class="text-sm text-ink-muted">
          <span class="font-mono text-ink">dopbase login --browser</span>
          printed it under the link.
        </p>
      </div>
      <DbCodeInput
        v-model="code"
        :alphabet="CODE_ALPHABET"
        :length="CODE_LENGTH"
        :error="codeError"
        autofocus
        @complete="lookUp" />
      <DbButton
        variant="primary"
        size="lg"
        type="submit"
        class="w-full"
        :loading="looking"
        :disabled="code.length !== CODE_LENGTH">
        Continue
      </DbButton>
      <p v-if="account" class="text-xs text-ink-muted">
        Signed in as <span class="text-ink">{{ account.email }}</span
        >. Not you?
        <button
          type="button"
          class="cursor-pointer text-accent-strong underline-offset-2 hover:underline"
          @click="switchAccount">
          Switch account
        </button>
      </p>
    </form>

    <!-- Review the request -->
    <div
      v-else-if="step === 'review' && request"
      class="flex flex-col gap-6"
      data-testid="cli-review">
      <div class="flex flex-col gap-2 text-center">
        <h1 class="text-2xl font-semibold text-ink-strong">
          {{
            isConfirmation
              ? "Let this terminal show secrets?"
              : "Sign in this terminal?"
          }}
        </h1>
        <p class="text-sm text-ink-muted">
          {{
            isConfirmation
              ? "It's signed in as you and wants to show secret values for the next 10 minutes."
              : "It will be signed in as you for up to 90 days."
          }}
        </p>
      </div>

      <div
        v-if="account"
        class="flex items-center gap-3 rounded-card border border-line bg-panel px-4 py-3"
        data-testid="cli-account">
        <div
          class="flex h-10 w-10 shrink-0 items-center justify-center rounded-control bg-accent/15 text-base font-semibold text-accent-strong"
          aria-hidden="true">
          {{ account.initial }}
        </div>
        <div class="flex min-w-0 flex-1 flex-col">
          <p class="text-xs text-ink-muted">Signed in as</p>
          <p
            class="truncate text-sm font-medium text-ink-strong"
            data-testid="cli-account-email">
            {{ account.email }}
          </p>
          <p class="text-xs text-ink-muted">{{ account.role }}</p>
        </div>
        <DbButton
          variant="secondary"
          size="sm"
          :disabled="deciding !== null"
          data-testid="cli-switch-account"
          @click="switchAccount">
          Switch
        </DbButton>
      </div>

      <div class="rounded-card border border-line bg-panel">
        <div
          class="border-b border-line-soft px-5 py-4 text-center font-mono text-3xl font-semibold tracking-[0.2em] text-ink-strong"
          data-testid="cli-review-code">
          {{ displayCode(code) }}
        </div>
        <dl class="grid grid-cols-[7rem_1fr] gap-y-2 px-5 py-4 text-sm">
          <dt class="text-ink-muted">Device</dt>
          <dd
            class="truncate font-mono text-ink-strong"
            :title="request.hostname ?? undefined">
            {{ request.hostname ?? "Not reported" }}
          </dd>
          <dt class="text-ink-muted">IP address</dt>
          <dd class="font-mono text-ink-strong">
            {{ request.ipAddress ?? "Not reported" }}
          </dd>
          <dt class="text-ink-muted">Requested</dt>
          <dd
            class="text-ink-strong"
            :title="formatDateTime(request.createdAt)">
            {{ formatRelativeTime(request.createdAt) }}
          </dd>
        </dl>
      </div>

      <p class="text-sm text-ink-muted">
        Only approve if the code matches your terminal and you ran the command
        yourself. If someone sent you this link, deny it, even when you're
        already signed in.
      </p>

      <DbAlert v-if="formError">{{ formError }}</DbAlert>

      <div class="flex flex-col gap-2.5">
        <DbButton
          variant="primary"
          size="lg"
          :loading="deciding === 'approve'"
          :disabled="deciding !== null"
          data-testid="cli-approve"
          @click="approve">
          {{ isConfirmation ? "Allow" : "Approve" }}
        </DbButton>
        <DbButton
          variant="secondary"
          size="lg"
          :loading="deciding === 'deny'"
          :disabled="deciding !== null"
          data-testid="cli-deny"
          @click="deny">
          Deny
        </DbButton>
      </div>
    </div>

    <!-- Done -->
    <div
      v-else
      class="flex flex-col items-center gap-5 text-center"
      data-testid="cli-done">
      <div
        class="flex h-14 w-14 items-center justify-center rounded-control"
        :class="
          step === 'approved' ? 'bg-ok/15 text-ok' : 'bg-crit/15 text-crit'
        ">
        <CheckIcon v-if="step === 'approved'" class="h-7 w-7" />
        <XIcon v-else class="h-7 w-7" />
      </div>
      <div class="flex flex-col gap-2">
        <h1 class="text-2xl font-semibold text-ink-strong">
          {{
            step === "approved"
              ? isConfirmation
                ? "Access confirmed"
                : "Terminal signed in"
              : "Request denied"
          }}
        </h1>
        <p class="text-sm text-ink-muted">
          {{
            step === "approved"
              ? account && !isConfirmation
                ? `It's signed in as ${account.email}. Go back to your terminal to carry on.`
                : "Go back to your terminal to carry on."
              : "The terminal was not signed in, and nothing was shared with it."
          }}
        </p>
      </div>

      <div
        v-if="step === 'approved' && closingIn !== null"
        class="flex w-full max-w-64 flex-col items-center gap-2.5"
        data-testid="cli-closing">
        <div class="h-1 w-full overflow-hidden rounded-full bg-line">
          <div
            class="close-bar h-full rounded-full bg-ok"
            :style="{ animationDuration: `${CLOSE_DELAY_SECONDS}s` }" />
        </div>
        <p class="text-xs text-ink-muted">
          This tab closes in {{ closingIn }}
          {{ closingIn === 1 ? "second" : "seconds" }}.
          <button
            type="button"
            class="cursor-pointer text-accent-strong underline-offset-2 hover:underline"
            data-testid="cli-keep-open"
            @click="keepOpen">
            Keep it open
          </button>
        </p>
      </div>
      <p
        v-else-if="step === 'approved'"
        role="status"
        class="text-xs text-ink-muted"
        data-testid="cli-close-note">
        {{
          closeBlocked
            ? "Your browser kept this tab open. You can close it now."
            : "You can close this tab."
        }}
      </p>

      <DbButton variant="ghost" @click="startOver">
        Enter another code
      </DbButton>
    </div>
    <ReauthModal />
  </FocusLayout>
</template>

<style scoped>
/* Drains once over the countdown, so it moves smoothly between ticks. */
.close-bar {
  transform-origin: left;
  animation: close-bar-drain linear forwards;
}

@keyframes close-bar-drain {
  from {
    transform: scaleX(1);
  }
  to {
    transform: scaleX(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .close-bar {
    animation: none;
  }
}
</style>
