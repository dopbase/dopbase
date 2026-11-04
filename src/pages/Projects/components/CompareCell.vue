<script setup lang="ts">
import { computed } from "vue";
import { CheckIcon, EyeOffIcon, PlusIcon } from "~/assets/icons";
import type { CellStatus } from "./ComparePanel.controller";

/**
 * CompareCell — one environment's state for one key, read against the
 * base. Says Set, Not set, Same or Differs, with the version where there
 * is one. Never shows a value. A fillable Not set cell is a button that
 * opens the way to set it.
 */
const props = withDefaults(
  defineProps<{
    status: CellStatus;
    version: number | null;
    fillable?: boolean;
  }>(),
  { fillable: false },
);

const look = computed(() => {
  switch (props.status) {
    case "base-missing":
    case "missing":
      return {
        label: "Not set",
        class: "border border-dashed border-crit/60 text-crit",
      };
    case "same":
      return { label: "Same", class: "border border-line-soft text-ink-faint" };
    case "differs":
      return {
        label: "Differs",
        class: "border border-accent/50 bg-accent-soft text-accent-strong",
      };
    case "unknown":
      return { label: "Set", class: "border border-line-soft text-ink-muted" };
    default:
      return { label: "Set", class: "bg-raised text-ink" };
  }
});

const showVersion = computed(
  () =>
    props.version !== null &&
    (props.status === "base-set" ||
      props.status === "set" ||
      props.status === "differs"),
);
</script>

<template>
  <component
    :is="fillable ? 'button' : 'span'"
    :type="fillable ? 'button' : undefined"
    class="flex h-8 w-full min-w-0 items-center gap-1.5 rounded-control px-2.5 text-left text-sm font-medium leading-5"
    :class="[
      look.class,
      fillable
        ? 'group cursor-pointer transition-colors hover:border-solid hover:bg-crit/10 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent pointer-coarse:h-10'
        : '',
    ]"
    :data-status="status"
    :data-fillable="fillable ? '' : undefined">
    <CheckIcon
      v-if="status === 'same'"
      class="h-3.5 w-3.5 shrink-0"
      aria-hidden="true" />
    <EyeOffIcon
      v-else-if="status === 'unknown'"
      class="h-3.5 w-3.5 shrink-0"
      aria-hidden="true" />
    <span class="truncate">{{ look.label }}</span>
    <span
      v-if="showVersion"
      class="ml-auto shrink-0 font-mono text-[13px] font-normal opacity-70">
      v{{ version }}
    </span>
    <span
      v-else-if="status === 'unknown'"
      class="ml-auto shrink-0 text-xs font-normal text-ink-faint">
      not compared
    </span>
    <span
      v-else-if="fillable"
      class="ml-auto flex shrink-0 items-center gap-1 text-xs font-normal text-crit/80 group-hover:text-crit">
      <PlusIcon class="h-3.5 w-3.5" aria-hidden="true" />
      Set
    </span>
  </component>
</template>
