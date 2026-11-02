<script setup lang="ts">
import { computed } from "vue";
import { CheckIcon } from "~/assets/icons";
import { formatList } from "~/utils/format";
import CliCommands from "./CliCommands.vue";
import EnvDropZone from "./EnvDropZone.vue";

/**
 * FirstRunChecklist — what a writer sees in an empty environment while no
 * environment in the project has secrets: the project exists, add its
 * secrets, then run the app through the CLI. It carries on the setup
 * timeline from Projects, with the same badges and connector. Renders
 * props and reports what was picked.
 */
const props = defineProps<{
  projectName: string;
  environmentName: string;
  /** Every environment in the project, for the "project created" step. */
  environmentNames: string[];
  /** Turns a dropped file into an import, while it's being read. */
  importing: boolean;
}>();

const emit = defineEmits<{
  "choose-file": [];
  "drop-file": [file: File];
  "paste-env": [];
  "add-one": [];
}>();

const environmentsSentence = computed(() => {
  const names = props.environmentNames;
  if (names.length <= 1)
    return `${props.projectName}, with ${names[0] ?? props.environmentName}. Add more environments from Settings.`;
  return `${props.projectName}, with ${formatList(names)}.`;
});

const importCommand = computed(() => [
  `dopbase import ${props.projectName}/${props.environmentName} .env`,
]);
const runCommands = computed(() => [
  "dopbase login",
  `dopbase run ${props.projectName}/${props.environmentName} -- npm run dev`,
]);
</script>

<template>
  <section
    class="flex max-w-[720px] flex-col gap-7 py-4"
    aria-labelledby="first-run-title"
    data-testid="first-run">
    <header class="flex flex-col gap-1.5">
      <h2
        id="first-run-title"
        class="text-lg font-semibold leading-[26px] text-ink-strong">
        Get {{ projectName }} running
      </h2>
      <p class="text-[15px] leading-6 text-ink-muted">
        A few minutes. This list goes away once
        <span class="font-mono">{{ environmentName }}</span> has its first
        secret.
      </p>
      <div class="mt-1.5 flex items-center gap-3">
        <div
          class="h-1 max-w-[600px] flex-1 overflow-hidden rounded-full bg-line"
          role="progressbar"
          aria-label="Setup progress"
          aria-valuemin="0"
          aria-valuemax="3"
          aria-valuenow="1">
          <div class="h-full w-1/3 rounded-full bg-ok" />
        </div>
        <span class="shrink-0 text-sm leading-[22px] text-ink-muted">
          1 of 3 done
        </span>
      </div>
    </header>

    <ol class="flex flex-col">
      <li class="relative flex gap-4 pb-6">
        <span
          class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-ok/40"
          aria-hidden="true" />
        <span
          class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
          aria-hidden="true">
          <CheckIcon class="h-4 w-4" />
        </span>
        <div class="min-w-0">
          <p class="text-[15px] font-semibold leading-5 text-ink-strong">
            Project created
            <span class="sr-only">(done)</span>
          </p>
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            {{ environmentsSentence }}
          </p>
        </div>
      </li>

      <li class="relative flex gap-4 pb-6" aria-current="step">
        <span
          class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-line"
          aria-hidden="true" />
        <span
          class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control border-[1.5px] border-ink-strong text-sm font-semibold text-ink-strong"
          aria-hidden="true">
          2
        </span>
        <div class="flex min-w-0 flex-1 flex-col gap-2.5">
          <div>
            <p class="text-[15px] font-semibold leading-5 text-ink-strong">
              Add secrets to
              <span class="font-mono">{{ environmentName }}</span>
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              Start with the .env you already have, or type them in.
            </p>
          </div>
          <EnvDropZone
            primary
            :importing="importing"
            @choose-file="emit('choose-file')"
            @drop-file="(file) => emit('drop-file', file)"
            @paste-env="emit('paste-env')"
            @add-one="emit('add-one')" />
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            Or from your terminal
          </p>
          <CliCommands :commands="importCommand" />
        </div>
      </li>

      <li class="flex gap-4">
        <span
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-control border border-line-strong text-sm font-semibold text-ink-muted"
          aria-hidden="true">
          3
        </span>
        <div class="flex min-w-0 flex-1 flex-col gap-2.5">
          <div>
            <p class="text-[15px] font-semibold leading-5 text-ink">
              Run your app with Dopbase
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-faint">
              Ready once
              <span class="font-mono">{{ environmentName }}</span>
              has a secret. Run it through the CLI and the secrets arrive as
              environment variables.
            </p>
          </div>
          <CliCommands :commands="runCommands" dim />
        </div>
      </li>
    </ol>
  </section>
</template>
