<script setup lang="ts">
import { useSsoLinkCardController } from "./SsoLinkCard.controller";
import { DbAlert, DbButton } from "~/components/ui";
import { formatDate } from "~/utils/format";

const {
  identity,
  visible,
  linking,
  unlinking,
  canUnlink,
  notice,
  error,
  link,
  unlink,
} = useSsoLinkCardController();
</script>

<template>
  <section
    v-if="visible && identity"
    class="flex flex-col gap-3 rounded-card border border-line bg-panel px-5 py-4"
    aria-labelledby="sso-heading"
    data-testid="sso-link-card">
    <div class="flex flex-wrap items-center gap-3">
      <div class="min-w-0 flex-1">
        <h2
          id="sso-heading"
          class="text-[15px] font-semibold leading-[22px] text-ink-strong">
          Single sign-on
        </h2>
        <p v-if="identity.linked" class="text-sm text-ink-muted">
          Linked to {{ identity.providerName }}
          <template v-if="identity.emailAtLink"
            >as {{ identity.emailAtLink }}</template
          ><template v-if="identity.linkedAt">
            on {{ formatDate(identity.linkedAt) }}</template
          >.
        </p>
        <p v-else class="text-sm text-ink-muted">
          Link your {{ identity.providerName }} account to sign in with it next
          time.
        </p>
      </div>
      <template v-if="identity.linked">
        <DbButton
          v-if="canUnlink"
          variant="ghost"
          :loading="unlinking"
          data-testid="sso-unlink"
          @click="unlink">
          Unlink
        </DbButton>
      </template>
      <DbButton v-else :loading="linking" @click="link">
        Link {{ identity.providerName }}
      </DbButton>
    </div>
    <DbAlert v-if="notice" tone="success">{{ notice }}</DbAlert>
    <DbAlert v-if="error">{{ error }}</DbAlert>
  </section>
</template>
