<script setup lang="ts">
import { computed, ref } from "vue";
import { highlightEnvContent, type EnvTokenType } from "~/utils/env-highlight";
import type { EnvFileIssue } from "~/utils/env-file";

/**
 * EnvFileEditor — a lightweight, dependency-free `.env` editor.
 *
 * A transparent textarea sits on top of a tokenized highlight overlay; both
 * layers share identical typography and wrapping so they stay aligned. The
 * overlay rows embed the line-number gutter, so wrapped lines keep the
 * gutter aligned. Malformed lines are highlighted and listed below.
 * Presentation-only: content is owned by the parent via v-model.
 */
const props = withDefaults(
  defineProps<{
    modelValue: string;
    issues?: EnvFileIssue[];
    disabled?: boolean;
    ariaLabel?: string;
    placeholder?: string;
  }>(),
  {
    issues: () => [],
    disabled: false,
    ariaLabel: "Edit .env content",
    placeholder: "",
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

/** Gutter width in `ch` units; the textarea's left padding must match. */
const GUTTER_CH = 6;
const MAX_SHOWN_ISSUES = 8;

const lineTokens = computed(() => highlightEnvContent(props.modelValue));
const errorLines = computed(() => new Set(props.issues.map((i) => i.line)));
const visibleIssues = computed(() => props.issues.slice(0, MAX_SHOWN_ISSUES));
const hiddenIssueCount = computed(() =>
  Math.max(0, props.issues.length - MAX_SHOWN_ISSUES),
);

const tokenClass: Record<EnvTokenType, string> = {
  plain: "text-ink",
  comment: "text-ink-faint italic",
  export: "text-ink-faint",
  key: "text-accent-strong",
  equals: "text-ink-faint",
  value: "text-ink-strong",
  quote: "text-ok",
  error: "text-crit underline decoration-crit/60 underline-offset-2",
};

const textarea = ref<HTMLTextAreaElement | null>(null);

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLTextAreaElement).value);
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== "Tab" || props.disabled) return;
  event.preventDefault();
  const el = event.target as HTMLTextAreaElement;
  const { selectionStart, selectionEnd } = el;
  const next =
    props.modelValue.slice(0, selectionStart) +
    "  " +
    props.modelValue.slice(selectionEnd);
  emit("update:modelValue", next);
  requestAnimationFrame(() => {
    el.selectionStart = el.selectionEnd = selectionStart + 2;
  });
}

function focus(): void {
  textarea.value?.focus();
}

defineExpose({ focus });
</script>

<template>
  <div
    data-testid="env-editor"
    class="flex flex-col overflow-hidden rounded-[var(--radius-card)] border bg-canvas transition-colors focus-within:border-accent"
    :class="issues.length > 0 ? 'border-crit/60' : 'border-line'">
    <div class="max-h-[26rem] min-h-[12rem] overflow-y-auto">
      <div class="relative">
        <!-- Highlight overlay: gutter number + tokens per logical line. -->
        <div
          aria-hidden="true"
          class="block w-full py-3 font-mono text-xs leading-relaxed">
          <div v-for="(tokens, line) in lineTokens" :key="line" class="flex">
            <span
              aria-hidden="true"
              class="w-[6ch] shrink-0 select-none pr-[1ch] text-right"
              :class="
                errorLines.has(line + 1)
                  ? 'font-semibold text-crit'
                  : 'text-ink-faint'
              "
              >{{ line + 1 }}</span
            ><span
              class="min-w-0 flex-1 whitespace-pre-wrap break-words pr-[1ch]"
              ><template v-if="tokens.length === 0">&#8203;</template
              ><span
                v-for="(token, tokenIndex) in tokens"
                :key="tokenIndex"
                :class="tokenClass[token.type]"
                >{{ token.text }}</span
              ></span
            >
          </div>
        </div>
        <!-- Input layer: transparent text, visible caret. -->
        <textarea
          ref="textarea"
          :value="modelValue"
          :disabled="disabled"
          :aria-label="ariaLabel"
          :placeholder="placeholder"
          :aria-invalid="issues.length > 0 ? true : undefined"
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          data-testid="env-editor-input"
          class="absolute inset-0 w-full resize-none overflow-hidden whitespace-pre-wrap break-words bg-transparent py-3 pr-[1ch] font-mono text-xs leading-relaxed text-transparent caret-accent-strong outline-none selection:bg-accent-soft placeholder:text-ink-faint disabled:opacity-50"
          :style="{ paddingLeft: `${GUTTER_CH}ch` }"
          @input="onInput"
          @keydown="onKeydown" />
      </div>
    </div>

    <!-- Validation issues -->
    <div
      v-if="issues.length > 0"
      data-testid="env-editor-issues"
      class="flex flex-col gap-1 border-t border-crit/30 bg-crit/5 px-3 py-2">
      <p
        v-for="issue in visibleIssues"
        :key="`${issue.line}-${issue.message}`"
        class="font-mono text-xs text-crit">
        Line {{ issue.line }}: {{ issue.message }}
      </p>
      <p v-if="hiddenIssueCount > 0" class="font-mono text-xs text-crit/80">
        +{{ hiddenIssueCount }} more issue(s)…
      </p>
    </div>
  </div>
</template>
