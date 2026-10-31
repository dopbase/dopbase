<script setup lang="ts">
import { SearchIcon } from "~/assets/icons";

/**
 * DbSearchField — a filter box with a search icon. Escape clears it.
 * Presentation-only.
 */
defineProps<{
  modelValue: string;
  placeholder: string;
  /** Names the field for screen readers when the placeholder isn't enough. */
  label?: string;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <label
    class="flex h-9 min-w-0 items-center gap-2 rounded-control border border-line bg-panel pl-2.5 pr-2 transition-colors focus-within:border-accent/60">
    <SearchIcon class="h-4 w-4 shrink-0 text-ink-faint" aria-hidden="true" />
    <input
      type="search"
      class="min-w-0 flex-1 bg-transparent text-sm leading-[22px] text-ink-strong outline-none placeholder:text-ink-faint [&::-webkit-search-cancel-button]:hidden"
      :value="modelValue"
      :placeholder="placeholder"
      :aria-label="label ?? placeholder"
      @input="
        emit('update:modelValue', ($event.target as HTMLInputElement).value)
      "
      @keydown.esc="modelValue && emit('update:modelValue', '')" />
    <slot name="trailing" />
  </label>
</template>
