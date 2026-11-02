<script setup lang="ts">
import { CheckIcon } from "~/assets/icons";

/**
 * DbCheckbox is a 20px box with a 2px keyline, check mark, focus halo,
 * and Dopbase purple active state. A visually hidden native input keeps
 * keyboard and a11y behavior. `indeterminate` shows a dash, for a "select
 * all" box when only some rows are chosen. Without a visible `label`, pass
 * `ariaLabel` so screen readers can name it.
 * Presentation-only.
 */
defineProps<{
  modelValue: boolean;
  label?: string;
  ariaLabel?: string;
  indeterminate?: boolean;
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
      :indeterminate="indeterminate && !modelValue"
      :disabled="disabled"
      :aria-label="label ? undefined : ariaLabel"
      @change="onChange" />
    <!-- Box: 20×20px with a 2px keyline. -->
    <span
      aria-hidden="true"
      class="flex h-5 w-5 shrink-0 items-center justify-center rounded-[5px] border-2 transition-colors duration-150 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent"
      :class="
        modelValue || indeterminate
          ? 'border-accent bg-accent/15'
          : 'border-line-strong'
      ">
      <CheckIcon v-if="modelValue" class="h-3.5 w-3.5 text-accent-strong" />
      <span
        v-else-if="indeterminate"
        class="h-0.5 w-2.5 rounded-full bg-accent-strong" />
    </span>
    <span v-if="label" class="text-sm text-ink">
      {{ label }}
    </span>
  </label>
</template>
