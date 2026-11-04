<script setup lang="ts">
import { computed } from "vue";
import {
  CLOSE_DELAY_SECONDS,
  CODE_ALPHABET,
  CODE_LENGTH,
  displayCode,
  useCliApproveController,
} from "./CliApprove.controller";
import { AuthLayout } from "~/layouts";
import ReauthModal from "~/components/app/ReauthModal.vue";
import {
  DbAlert,
  DbButton,
  DbCode,
  DbCodeInput,
  DbSpinner,
} from "~/components/ui";
import {
  CheckIcon,
  ClockIcon,
  ExternalLinkIcon,
  ServerIcon,
  TerminalIcon,
  XIcon,
} from "~/assets/icons";
import { formatDateTime, formatRelativeTime } from "~/utils/format";

/**
 * CLI approval — where `dopbase login --browser` sends the person to
 * approve the code their terminal shows. It names the machine, client,
 * address and time the request came from, mirrors what the terminal should
 * show, and keeps the account it signs in on screen.
 */
const {
  step,
  code,
  request,
  account,
  isConfirmation,
  machine,
  sessionDays,
  expiresIn,
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

const host = window.location.host;
/** "Just now", capitalised to start the line. */
const requestedAgo = computed(() => {
  const text = request.value ? formatRelativeTime(request.value.createdAt) : "";
  return text.charAt(0).toUpperCase() + text.slice(1);
});
const characters = computed(() => [...code.value.padEnd(CODE_LENGTH, " ")]);
const approveLabel = computed(() =>
  isConfirmation.value
    ? "Allow"
    : machine.value
      ? `Approve ${machine.value}`
      : "Approve",
);
</script>

<template>
  <AuthLayout>
    <!-- Checking a code that came in the link -->
    <div
      v-if="step === 'checking'"
      class="flex items-center gap-3 text-sm text-ink-muted"
      data-testid="cli-checking">
      <DbSpinner />
      {{ code.length === CODE_LENGTH ? "Checking the code…" : "One moment…" }}
    </div>

    <!-- Enter the code -->
    <form
      v-else-if="step === 'enter'"
      class="flex flex-col gap-6"
      novalidate
      data-testid="cli-code-form"
      @submit.prevent="lookUp">
      <div>
        <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
          Approve CLI sign-in
        </h1>
        <p class="mt-2 text-sm text-ink-muted">
          Enter the code
          <span class="font-mono text-ink">dopbase login --browser</span>
          printed in your terminal.
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
        type="submit"
        :loading="looking"
        :disabled="code.length !== CODE_LENGTH">
        Continue
      </DbButton>
      <p v-if="account" class="text-[13px] text-ink-muted">
        Signed in as <span class="text-ink">{{ account.email }}</span
        >. Not you?
        <button
          type="button"
          class="cursor-pointer underline underline-offset-4 hover:text-ink"
          @click="switchAccount">
          Switch account
        </button>
      </p>
    </form>

    <!-- Review the request -->
    <div
      v-else-if="step === 'review' && request"
      class="flex flex-col gap-5"
      data-testid="cli-review">
      <div>
        <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
          {{
            isConfirmation
              ? "Let this terminal show secrets?"
              : "Approve CLI sign-in"
          }}
        </h1>
        <p class="mt-2 text-sm text-ink-muted">
          Check that this code matches the one in your terminal.
        </p>
      </div>

      <div
        class="flex items-center gap-1.5"
        role="img"
        :aria-label="`Code ${displayCode(code)}`"
        data-testid="cli-review-code">
        <template v-for="(character, index) in characters" :key="index">
          <span
            v-if="index === 4"
            class="px-0.5 text-ink-faint"
            aria-hidden="true">
            –
          </span>
          <span
            class="flex h-11 w-9 items-center justify-center rounded-control border border-line bg-raised font-mono text-lg font-semibold text-ink-strong"
            aria-hidden="true">
            {{ character }}
          </span>
        </template>
      </div>
      <button
        type="button"
        class="-mt-2 self-start cursor-pointer text-[13px] text-crit underline underline-offset-4"
        :disabled="deciding !== null"
        data-testid="cli-mismatch"
        @click="deny">
        It doesn't match
      </button>

      <div class="rounded-card border border-line bg-panel px-4 py-3">
        <p class="text-[13px] text-ink-muted">Requested by</p>
        <dl class="mt-2 flex flex-col gap-2 text-[13px]">
          <div class="flex items-center gap-2">
            <ServerIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            <dt class="text-ink-muted">Machine</dt>
            <dd class="ml-auto truncate font-mono text-ink-strong">
              {{ request.hostname ?? "Not reported" }}
            </dd>
          </div>
          <div class="flex items-center gap-2">
            <TerminalIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            <dt class="text-ink-muted">Client</dt>
            <dd
              class="ml-auto truncate text-ink-strong"
              data-testid="cli-client">
              {{ request.client ?? "Not reported" }}
            </dd>
          </div>
          <div class="flex items-center gap-2">
            <ExternalLinkIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            <dt class="text-ink-muted">From</dt>
            <dd class="ml-auto truncate font-mono text-ink-strong">
              {{ request.ipAddress ?? "Not reported" }}
            </dd>
          </div>
          <div class="flex items-center gap-2">
            <ClockIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            <dt class="text-ink-muted">Requested</dt>
            <dd
              class="ml-auto text-ink-strong"
              :title="formatDateTime(request.createdAt)">
              {{ requestedAgo }}.
              <span v-if="expiresIn">Expires in {{ expiresIn }}</span>
            </dd>
          </div>
        </dl>
        <p
          v-if="account"
          class="mt-3 border-t border-line-soft pt-3 text-[13px] leading-relaxed text-ink"
          data-testid="cli-account">
          <template v-if="isConfirmation">
            Lets the terminal signed in as
            <span data-testid="cli-account-email">{{ account.email }}</span>
            show secret values for the next 10 minutes.
          </template>
          <template v-else>
            Signs in as
            <span data-testid="cli-account-email">{{ account.email }}</span>
            with your access, for {{ sessionDays }} days on this machine. You
            can end it from Account at any time.
          </template>
          <button
            type="button"
            class="ml-1 cursor-pointer text-ink-muted underline underline-offset-4 hover:text-ink"
            :disabled="deciding !== null"
            data-testid="cli-switch-account"
            @click="switchAccount">
            Not you?
          </button>
        </p>
      </div>

      <DbAlert v-if="formError">{{ formError }}</DbAlert>

      <div class="grid grid-cols-2 gap-2">
        <DbButton
          :loading="deciding === 'deny'"
          :disabled="deciding !== null"
          data-testid="cli-deny"
          @click="deny">
          Deny
        </DbButton>
        <DbButton
          variant="primary"
          :loading="deciding === 'approve'"
          :disabled="deciding !== null"
          data-testid="cli-approve"
          @click="approve">
          <span class="truncate">{{ approveLabel }}</span>
        </DbButton>
      </div>
    </div>

    <!-- Expired while on screen -->
    <div
      v-else-if="step === 'expired'"
      class="flex flex-col gap-4"
      data-testid="cli-expired">
      <span
        class="flex h-10 w-10 items-center justify-center rounded-control bg-crit/15 text-crit">
        <ClockIcon class="h-5 w-5" />
      </span>
      <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
        This request expired
      </h1>
      <p class="text-sm leading-relaxed text-ink">
        Codes last a few minutes. Run the command again to get a new one.
        Nothing was signed in.
      </p>
      <DbCode>dopbase login --browser</DbCode>
      <DbButton class="self-start" @click="startOver"
        >Enter a new code</DbButton
      >
    </div>

    <!-- Approved or denied -->
    <div v-else class="flex flex-col gap-4" data-testid="cli-done">
      <span
        class="flex h-10 w-10 items-center justify-center rounded-control"
        :class="
          step === 'approved' ? 'bg-ok/15 text-ok' : 'bg-crit/15 text-crit'
        ">
        <CheckIcon v-if="step === 'approved'" class="h-5 w-5" />
        <XIcon v-else class="h-5 w-5" />
      </span>
      <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
        {{
          step === "approved"
            ? isConfirmation
              ? "Access confirmed"
              : `${machine ?? "The terminal"} is signed in`
            : "Request denied"
        }}
      </h1>
      <p class="text-sm leading-relaxed text-ink">
        {{
          step === "approved"
            ? "Go back to your terminal to carry on."
            : "The terminal was not signed in, and nothing was shared with it. If you didn't start this, tell your admin."
        }}
      </p>
      <div
        v-if="step === 'approved' && account && !isConfirmation"
        class="rounded-card border border-line bg-panel px-4 py-3 font-mono text-[13px]">
        <p class="flex items-center gap-2 text-ink-strong">
          <CheckIcon class="h-4 w-4 text-ok" />
          Signed in as {{ account.email }}
        </p>
        <p class="mt-1 text-ink-muted">Session lasts {{ sessionDays }} days</p>
      </div>
      <div
        v-if="step === 'approved' && closingIn !== null"
        class="flex flex-col gap-2"
        data-testid="cli-closing">
        <div
          class="h-1 w-full max-w-64 overflow-hidden rounded-control bg-line">
          <div
            class="close-bar h-full bg-ok"
            :style="{ animationDuration: `${CLOSE_DELAY_SECONDS}s` }" />
        </div>
        <p class="text-[13px] text-ink-muted">
          This tab closes in {{ closingIn }}
          {{ closingIn === 1 ? "second" : "seconds" }}.
          <button
            type="button"
            class="cursor-pointer underline underline-offset-4 hover:text-ink"
            data-testid="cli-keep-open"
            @click="keepOpen">
            Keep it open
          </button>
        </p>
      </div>
      <p
        v-else-if="step === 'approved'"
        role="status"
        class="text-[13px] text-ink-muted"
        data-testid="cli-close-note">
        {{
          closeBlocked
            ? "Your browser kept this tab open. You can close it now."
            : "You can close this tab."
        }}
      </p>
      <DbButton variant="ghost" class="self-start" @click="startOver">
        Enter another code
      </DbButton>
    </div>

    <template #footer>
      <span>Didn't run dopbase login? Deny, then tell your admin.</span>
    </template>

    <!-- What the terminal should show, so both sides can be compared. -->
    <template #aside>
      <div class="flex flex-1 flex-col justify-center px-16 py-12">
        <p class="text-[13px] text-ink-muted">What your terminal should show</p>
        <div
          class="mt-3 max-w-[600px] overflow-hidden rounded-card border border-line bg-canvas font-mono text-[13px]"
          data-testid="cli-terminal">
          <div
            class="flex items-center justify-between border-b border-line-soft px-4 py-2.5 text-ink-muted">
            <span class="flex items-center gap-2">
              <TerminalIcon class="h-3.5 w-3.5" />
              {{ machine ?? "your terminal" }}
            </span>
          </div>
          <div class="flex flex-col gap-2 px-4 py-4 leading-relaxed">
            <p class="text-ink-strong">$ dopbase login --browser</p>
            <p class="text-ink">
              Opening {{ host }}/cli/approve in your browser.
            </p>
            <p class="text-ink-muted">
              If it didn't open, go there and confirm this code:
            </p>
            <p>
              <span
                class="rounded-control bg-raised px-2 py-1 font-semibold text-ink-strong">
                {{
                  code.length === CODE_LENGTH ? displayCode(code) : "XXXX-XXXX"
                }}
              </span>
            </p>
            <p class="flex items-center gap-2 text-ink-muted">
              <span class="h-1.5 w-1.5 rounded-control bg-accent" />
              {{
                step === "approved"
                  ? "Signed in."
                  : expiresIn
                    ? `Waiting for approval. Expires in ${expiresIn}.`
                    : "Waiting for approval."
              }}
            </p>
          </div>
        </div>
        <p class="mt-3 max-w-[600px] text-[13px] text-ink-muted">
          Same code on both sides means it's your terminal asking. If you didn't
          start this, deny it.
        </p>
      </div>
    </template>
  </AuthLayout>
  <ReauthModal />
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
