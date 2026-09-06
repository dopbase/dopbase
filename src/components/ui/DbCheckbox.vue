<script setup lang="ts">
import { CheckIcon } from "~/assets/icons";

/**
 * DbCheckbox — PocketBase-style checkbox (20px box, 2px keyline, check
 * mark + focus halo) with the Dopbase purple active state. A
 * visually-hidden native input keeps keyboard and a11y behavior.
 * Presentation-only.
 */
defineProps<{
  modelValue: boolean;
  label?: string;
  disabled?: boolean;
}>();

const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

function onChange(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).checked);
}
</script>

<template>
  <label
    class="inline-flex select-none items-center gap-2.5"
    :class="disabled ? 'cursor-default opacity-50' : 'cursor-pointer'">
    <input
      type="checkbox"
      class="peer sr-only"
      :checked="modelValue"
      :disabled="disabled"
      :aria-label="label ? undefined : label"
      @change="onChange" />
    <!-- Box: 20×20px with a 2px keyline (PocketBase checkbox geometry). -->
    <span
      aria-hidden="true"
      class="flex h-5 w-5 shrink-0 items-center justify-center rounded-[5px] border-2 transition-colors duration-150 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent"
      :class="modelValue ? 'border-accent bg-accent/15' : 'border-line-strong'">
      <CheckIcon v-if="modelValue" class="h-3.5 w-3.5 text-accent-strong" />
    </span>
    <span v-if="label" class="text-sm text-ink">
      {{ label }}
    </span>
  </label>
</template>
