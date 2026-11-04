<script setup lang="ts">
import { computed } from "vue";
import { CheckIcon, DopbaseIcon } from "~/assets/icons";

/**
 * SetupSteps — the setup checklist beside the form: which step this is and
 * what comes after, over a large brand mark cropped by the bottom-right
 * corner. Presentation-only.
 */
const props = defineProps<{
  host: string;
  /** 1 while creating root, 2 on the recovery kit. */
  current: 1 | 2;
}>();

const steps = computed(() => [
  {
    title: "Root account",
    body: "The one account that can always sign in, even if single sign-on breaks.",
  },
  {
    title: "Recovery kit",
    body: "The master key. Without it, no backup of this server can be restored.",
  },
  {
    title: "Sign-in for your team",
    body: "Google, passwords or both. You can change this later.",
  },
  {
    title: "First project",
    body: "Import a .env and run your app with it.",
  },
]);

function state(index: number): "done" | "current" | "next" {
  const number = index + 1;
  if (number < props.current) return "done";
  return number === props.current ? "current" : "next";
}
</script>

<template>
  <div class="relative flex flex-1 flex-col px-20 pb-12 pt-24">
    <!-- Sized and placed as in the Figma frame: 521px wide, hanging off the
         bottom-right corner, which the aside's overflow-hidden crops. -->
    <DopbaseIcon
      class="pointer-events-none absolute -bottom-[259px] -right-[171px] w-[521px] opacity-90"
      data-testid="setup-brand-mark" />
    <p class="relative text-[13px] text-ink-muted">Setting up</p>
    <p class="relative mt-1 font-mono text-[17px] text-ink-strong">
      {{ host }}
    </p>
    <ol
      class="relative mt-7 flex max-w-[420px] flex-col"
      aria-label="Setup steps">
      <li
        v-for="(step, index) in steps"
        :key="step.title"
        class="relative flex gap-4 pb-7 last:pb-0"
        :aria-current="state(index) === 'current' ? 'step' : undefined">
        <span
          v-if="index < steps.length - 1"
          class="absolute left-[13px] top-7 h-[calc(100%-28px)] w-[1.5px]"
          :class="state(index) === 'done' ? 'bg-ok' : 'bg-line'"
          aria-hidden="true" />
        <span
          class="relative z-10 flex h-7 w-7 shrink-0 items-center justify-center rounded-full border text-xs font-medium"
          :class="{
            'border-ok/50 bg-ok/15 text-ok': state(index) === 'done',
            'border-ink-strong bg-ink-strong text-canvas':
              state(index) === 'current',
            'border-line bg-canvas text-ink-muted': state(index) === 'next',
          }">
          <CheckIcon v-if="state(index) === 'done'" class="h-3.5 w-3.5" />
          <template v-else>{{ index + 1 }}</template>
        </span>
        <div>
          <p
            class="text-sm font-medium"
            :class="
              state(index) === 'next' ? 'text-ink-muted' : 'text-ink-strong'
            ">
            {{ step.title }}
          </p>
          <p
            class="mt-1 text-[13px] leading-relaxed"
            :class="
              state(index) === 'next' ? 'text-ink-faint' : 'text-ink-muted'
            ">
            {{ step.body }}
          </p>
        </div>
      </li>
    </ol>
  </div>
</template>
