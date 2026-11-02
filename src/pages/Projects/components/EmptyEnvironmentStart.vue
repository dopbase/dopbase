<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { LayersIcon } from "~/assets/icons";
import { DbButton, DbSegment, DbSelect } from "~/components/ui";
import CliCommands from "./CliCommands.vue";
import EnvDropZone from "./EnvDropZone.vue";
import type { CopySource } from "./EmptyEnvironment.controller";

/**
 * EmptyEnvironmentStart — what a writer sees in an empty environment when
 * another environment in the project already has secrets. Copying its
 * keys comes first, with values left empty unless asked for, above the
 * same drop zone and import command as the first-run checklist. Renders
 * props and reports what was picked.
 */
const props = defineProps<{
  projectName: string;
  environmentName: string;
  /** Environments with secrets, most first. Never empty. */
  sources: CopySource[];
  /** Turns a dropped file into an import, while it's being read. */
  importing: boolean;
}>();

const emit = defineEmits<{
  review: [source: CopySource, values: boolean];
  "choose-file": [];
  "drop-file": [file: File];
  "paste-env": [];
  "add-one": [];
}>();

const sourceId = ref(props.sources[0]?.id ?? "");
const copyWhat = ref<"keys" | "values">("keys");

watch(
  () => props.sources,
  (sources) => {
    if (!sources.some((source) => source.id === sourceId.value))
      sourceId.value = sources[0]?.id ?? "";
  },
);

const source = computed(
  () =>
    props.sources.find((candidate) => candidate.id === sourceId.value) ??
    props.sources[0]!,
);
const sourceOptions = computed(() =>
  props.sources.map((candidate) => ({
    value: candidate.id,
    label: `${candidate.name} · ${secretsLabel(candidate.secretCount)}`,
  })),
);
const copyOptions = [
  { value: "keys", label: "Keys only" },
  { value: "values", label: "Keys and values" },
];
const copiesValues = computed(
  () => copyWhat.value === "values" && source.value.canCopyValues,
);
const importCommand = computed(() => [
  `dopbase import ${props.projectName}/${props.environmentName} .env`,
]);

function secretsLabel(count: number): string {
  return count === 1 ? "1 secret" : `${count} secrets`;
}

function keysLabel(count: number): string {
  return count === 1 ? "1 key" : `${count} keys`;
}
</script>

<template>
  <section
    class="flex max-w-[720px] flex-col gap-5 py-4"
    aria-labelledby="empty-start-title"
    data-testid="empty-start">
    <header class="flex flex-col gap-1.5">
      <h2
        id="empty-start-title"
        class="text-lg font-semibold leading-[26px] text-ink-strong">
        <span class="font-mono">{{ environmentName }}</span> has no secrets yet
      </h2>
      <p class="text-[15px] leading-6 text-ink-muted">
        <span class="font-mono">{{ source.name }}</span> already has
        {{ source.secretCount }}. Start from its keys, import a file, or add
        them yourself.
      </p>
    </header>

    <div
      class="flex flex-col gap-3.5 rounded-card border border-accent/50 bg-panel p-5"
      data-testid="copy-from">
      <p
        class="flex items-center gap-2 text-sm font-semibold leading-5 text-ink-strong">
        <LayersIcon class="h-4 w-4 text-accent-strong" />
        Copy from another environment
      </p>
      <p class="text-sm leading-[22px] text-ink-muted">
        Every key comes over, so
        <span class="font-mono">{{ environmentName }}</span> has nothing
        missing. You fill in the values that should differ.
      </p>
      <DbSelect v-model="sourceId" label="From" :options="sourceOptions" />
      <DbSegment
        v-if="source.canCopyValues"
        v-model="copyWhat"
        class="self-start"
        :options="copyOptions"
        label="What to copy" />
      <p class="text-sm leading-[22px] text-ink-faint">
        <template v-if="!source.canCopyValues">
          You can see the keys in
          <span class="font-mono">{{ source.name }}</span> but not its values,
          so they start empty.
        </template>
        <template v-else-if="copiesValues">
          Values are copied on the server. Change the ones
          <span class="font-mono">{{ environmentName }}</span> shouldn't share.
        </template>
        <template v-else>
          Values start empty, so
          <span class="font-mono">{{ environmentName }}</span> never points at
          <span class="font-mono">{{ source.name }}</span
          >'s database by accident.
        </template>
      </p>
      <DbButton
        variant="primary"
        class="self-start"
        data-testid="review-keys"
        @click="emit('review', source, copiesValues)">
        Review {{ keysLabel(source.secretCount) }}
      </DbButton>
    </div>

    <EnvDropZone
      :importing="importing"
      @choose-file="emit('choose-file')"
      @drop-file="(file) => emit('drop-file', file)"
      @paste-env="emit('paste-env')"
      @add-one="emit('add-one')" />

    <div class="flex flex-col gap-2.5">
      <p class="text-sm leading-[22px] text-ink-muted">Or from your terminal</p>
      <CliCommands :commands="importCommand" />
    </div>
  </section>
</template>
