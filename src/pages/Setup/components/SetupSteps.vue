<script setup lang="ts">
import { computed } from "vue";
import { CheckIcon } from "~/assets/icons";

/**
 * SetupSteps — the setup checklist beside the form: which step this is and
 * what comes after. Steps three and four happen in the console, so they
 * say where. Presentation-only.
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
    body: "Passwords, single sign-on or both, from Single sign-on in the console.",
  },
  {
    title: "First project",
    body: "Create a project, import a .env, and run your app with it.",
  },
]);

function state(index: number): "done" | "current" | "next" {
  const number = index + 1;
  if (number < props.current) return "done";
  return number === props.current ? "current" : "next";
}
</script>

<template>
  <div class="flex flex-1 flex-col justify-center px-16 py-12">
    <p class="text-[13px] text-ink-muted">Setting up</p>
    <p class="mt-1 font-mono text-[17px] text-ink-strong">{{ host }}</p>
    <ol class="mt-8 flex max-w-[400px] flex-col" aria-label="Setup steps">
      <li
        v-for="(step, index) in steps"
        :key="step.title"
        class="relative flex gap-4 pb-7 last:pb-0"
        :aria-current="state(index) === 'current' ? 'step' : undefined">
        <span
          v-if="index < steps.length - 1"
          class="absolute left-[13px] top-7 h-[calc(100%-28px)] w-px"
          :class="state(index) === 'done' ? 'bg-ok' : 'bg-line'"
          aria-hidden="true" />
        <span
          class="relative z-10 flex h-[27px] w-[27px] shrink-0 items-center justify-center rounded-control border text-xs font-medium"
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
