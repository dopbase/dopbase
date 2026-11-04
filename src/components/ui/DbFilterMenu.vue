<script setup lang="ts">
import { computed } from "vue";
import DbMenu from "./DbMenu.vue";
import DbMenuItem from "./DbMenuItem.vue";
import { PlusIcon, XIcon } from "~/assets/icons";

/**
 * DbFilterMenu — a filter chip. Empty, it reads "+ Role" and opens the
 * choices; set, it reads "Role: Admin" with a × to clear it. Presentation-
 * only: the page filters.
 */
const props = defineProps<{
  label: string;
  options: readonly { value: string; label: string }[];
  modelValue: string | null;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string | null] }>();

const chosen = computed(
  () =>
    props.options.find((option) => option.value === props.modelValue) ?? null,
);
</script>

<template>
  <span
    v-if="chosen"
    class="inline-flex h-8 items-center gap-1 rounded-control border border-accent/50 bg-accent-soft pl-2.5 pr-1 text-sm text-ink-strong"
    data-testid="filter-chip">
    <span class="text-ink-muted">{{ label }}:</span>
    {{ chosen.label }}
    <button
      type="button"
      class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong pointer-coarse:h-8 pointer-coarse:w-8"
      :aria-label="`Clear ${label} filter`"
      @click="emit('update:modelValue', null)">
      <XIcon class="h-3.5 w-3.5" />
    </button>
  </span>
  <DbMenu v-else align="start">
    <template #trigger="{ toggle, attrs }">
      <button
        type="button"
        class="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-control border border-dashed border-line-strong px-2.5 text-sm text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
        v-bind="attrs"
        :aria-label="`Filter by ${label}`"
        @click="toggle">
        <PlusIcon class="h-3.5 w-3.5" />
        {{ label }}
      </button>
    </template>
    <DbMenuItem
      v-for="option in options"
      :key="option.value"
      @select="emit('update:modelValue', option.value)">
      {{ option.label }}
    </DbMenuItem>
  </DbMenu>
</template>
