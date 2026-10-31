<script setup lang="ts">
import { computed, ref } from "vue";
import { CheckIcon, UploadIcon } from "~/assets/icons";
import { DbButton, DbCopyButton } from "~/components/ui";

/**
 * FirstRunChecklist — what an empty environment shows to someone who can
 * write there: the project exists, add its secrets, then run the app
 * through the CLI. A .env file can be chosen, dropped or pasted. Renders
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

const dragging = ref(false);

const environmentsSentence = computed(() => {
  const names = props.environmentNames;
  if (names.length <= 1)
    return `${props.projectName}, with ${names[0] ?? props.environmentName}.`;
  return `${props.projectName}, with ${names.slice(0, -1).join(", ")} and ${names.at(-1)}.`;
});

const commands = computed(() => [
  "dopbase login",
  `dopbase run ${props.projectName}/${props.environmentName} -- npm run dev`,
]);

function onDrop(event: DragEvent): void {
  dragging.value = false;
  const file = event.dataTransfer?.files?.[0];
  if (file) emit("drop-file", file);
}
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
        Get {{ projectName }} running with Dopbase
      </h2>
      <p class="text-[15px] leading-6 text-ink-muted">
        About three minutes. This list goes away once
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

    <ol class="flex flex-col gap-7">
      <li class="flex gap-4">
        <span
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
          aria-hidden="true">
          <CheckIcon class="h-4 w-4" />
        </span>
        <div class="min-w-0">
          <p class="text-[15px] font-semibold leading-5 text-ink-strong">
            Create the project
            <span class="sr-only">(done)</span>
          </p>
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            {{ environmentsSentence }}
          </p>
        </div>
      </li>

      <li class="flex gap-4">
        <span
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-control border-[1.5px] border-ink-strong text-sm font-semibold text-ink-strong"
          aria-hidden="true">
          2
        </span>
        <div class="flex min-w-0 flex-1 flex-col gap-2.5">
          <div>
            <p class="text-[15px] font-semibold leading-5 text-ink-strong">
              Add your secrets to
              <span class="font-mono">{{ environmentName }}</span>
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              Start with the .env you already have.
            </p>
          </div>
          <div
            class="flex flex-col items-center gap-2 rounded-card border border-dashed px-6 py-6 text-center transition-colors"
            :class="
              dragging
                ? 'border-accent/60 bg-accent-soft'
                : 'border-line bg-panel'
            "
            data-testid="first-run-drop"
            @dragover.prevent="dragging = true"
            @dragleave.prevent="dragging = false"
            @drop.prevent="onDrop">
            <UploadIcon class="h-4 w-4 text-ink-muted" />
            <p class="text-sm font-medium leading-5 text-ink-strong">
              {{
                importing
                  ? "Reading the file…"
                  : "Drop a .env file here, or paste its contents"
              }}
            </p>
            <p class="text-sm leading-[22px] text-ink-muted">
              Values are encrypted before they're stored. The file itself isn't
              kept.
            </p>
            <div class="mt-1 flex flex-wrap justify-center gap-2">
              <DbButton
                variant="primary"
                :disabled="importing"
                @click="emit('choose-file')">
                Choose a file
              </DbButton>
              <DbButton :disabled="importing" @click="emit('paste-env')">
                Paste .env
              </DbButton>
            </div>
          </div>
          <button
            type="button"
            class="cursor-pointer self-start text-sm leading-[22px] text-ink underline underline-offset-2 hover:text-ink-strong pointer-coarse:py-2.5"
            @click="emit('add-one')">
            Or add secrets one at a time
          </button>
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
            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              Once
              <span class="font-mono">{{ environmentName }}</span>
              has secrets, start your app through the CLI.
            </p>
          </div>
          <div
            class="flex flex-col gap-1.5 rounded-card border border-line-soft bg-editor px-4 py-3">
            <div
              v-for="command in commands"
              :key="command"
              class="flex items-center gap-2">
              <code
                class="min-w-0 flex-1 truncate font-mono text-sm leading-[22px] text-ink-strong"
                :title="command">
                <span class="select-none text-ink-muted">$ </span>{{ command }}
              </code>
              <DbCopyButton :value="command" label="Copy command" icon-only />
            </div>
          </div>
        </div>
      </li>
    </ol>
  </section>
</template>
