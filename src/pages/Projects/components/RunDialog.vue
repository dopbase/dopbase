<script setup lang="ts">
import { computed, toRef } from "vue";
import { RUN_TARGETS, useRunDialogController } from "./RunDialog.controller";
import { KeyIcon } from "~/assets/icons";
import { DbButton, DbCopyButton, DbModal, DbSegment } from "~/components/ui";
import type { Environment } from "~/services";

/**
 * RunDialog — opened by Run in the context bar. Shows how to run an app with
 * this environment's values from the CLI, Docker or GitHub Actions, and the
 * runner token deploys use.
 */
const props = defineProps<{
  open: boolean;
  projectName: string;
  environment: Environment;
  /** Show the runner token summary. Only managers can list tokens. */
  canManage: boolean;
}>();

const emit = defineEmits<{ close: [] }>();

const tokensLink = computed(() => ({
  name: "environment-tokens",
  params: {
    projectRef: props.projectName,
    environmentName: props.environment.name,
  },
}));

const {
  target,
  command,
  hint,
  defaultCommand,
  tokensLoaded,
  deployToken,
  deployTokenSummary,
} = useRunDialogController(
  toRef(props, "projectName"),
  toRef(props, "environment"),
  toRef(props, "canManage"),
  toRef(props, "open"),
);
</script>

<template>
  <DbModal
    :open="open"
    :title="`Run with ${environment.name} values`"
    @close="emit('close')">
    <div class="flex flex-col gap-3.5" data-testid="run-dialog">
      <DbSegment v-model="target" :options="RUN_TARGETS" label="Run with" />

      <div
        class="flex items-start gap-2 rounded-control border border-line-soft bg-editor py-3 pl-3 pr-1.5">
        <pre
          class="min-w-0 flex-1 overflow-x-auto font-mono text-[13px] leading-[22px] text-ink-strong"
          :class="target === 'cli' ? 'whitespace-pre-wrap' : 'whitespace-pre'"
          data-testid="run-command"><code>{{ command }}</code></pre>
        <DbCopyButton :value="command" label="Copy command" icon-only />
      </div>

      <p class="text-[13px] leading-[22px] text-ink-muted">{{ hint }}</p>

      <p
        v-if="target === 'cli'"
        class="text-[13px] leading-[22px] text-ink-muted">
        Tip: run
        <code class="font-mono text-ink">{{ defaultCommand }}</code>
        once, and a plain
        <code class="font-mono text-ink">dopbase run -- npm start</code>
        picks this environment.
      </p>

      <template v-if="canManage && tokensLoaded">
        <div class="h-px bg-line-soft" />
        <p class="text-sm font-medium leading-5 text-ink">
          Deploys read
          <span class="font-mono">{{ environment.name }}</span>
          with a runner token
        </p>
        <template v-if="deployToken">
          <div class="flex items-center gap-2">
            <KeyIcon class="h-4 w-4 shrink-0 text-ink-muted" />
            <span
              class="min-w-0 truncate font-mono text-[13px] leading-[22px] text-ink">
              {{ deployToken.name }}
            </span>
            <RouterLink
              :to="tokensLink"
              class="ml-auto shrink-0 text-[13px] leading-[22px] text-ink underline underline-offset-2 hover:text-ink-strong"
              @click="emit('close')">
              Manage tokens
            </RouterLink>
          </div>
          <p
            class="text-[13px] leading-[22px] text-ink-muted"
            data-testid="deploy-token-summary">
            {{ deployTokenSummary }}
          </p>
        </template>
        <p v-else class="text-[13px] leading-[22px] text-ink-muted">
          No runner token here yet.
          <RouterLink
            :to="tokensLink"
            class="text-ink underline underline-offset-2 hover:text-ink-strong"
            @click="emit('close')">
            Create one
          </RouterLink>
          for your deploys.
        </p>
      </template>
    </div>
    <template #footer>
      <DbButton variant="primary" @click="emit('close')">Done</DbButton>
    </template>
  </DbModal>
</template>
