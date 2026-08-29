<script setup lang="ts">
import { useId } from "vue";

/**
 * DbSelect — labeled native select styled for the dark palette.
 * Presentation-only.
 */
defineProps<{
  modelValue: string;
  label?: string;
  options: Array<{ label: string; value: string }>;
  disabled?: boolean;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const id = useId();

function onChange(event: Event): void {
  emit("update:modelValue", (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <label v-if="label" :for="id" class="text-xs font-medium text-ink-muted">
      {{ label }}
    </label>
    <select
      :id="id"
      :value="modelValue"
      :disabled="disabled"
      class="w-full cursor-pointer rounded-md border border-line bg-canvas px-3 py-2 text-sm text-ink-strong outline-none transition-colors focus:border-accent disabled:opacity-50"
      @change="onChange">
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value">
        {{ option.label }}
      </option>
    </select>
  </div>
</template>
