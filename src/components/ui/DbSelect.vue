<script setup lang="ts">
import { useId } from "vue";
import { ChevronDownIcon } from "~/assets/icons";

/**
 * DbSelect — labeled native select styled as a PocketBase field: the
 * label sits inside the filled control and a chevron hints at the menu.
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
    <!-- PocketBase-style field: label inside the filled select, custom
         chevron, fill lightens on focus. -->
    <div
      class="rounded-control bg-raised transition-colors duration-150 focus-within:bg-line"
      :class="disabled ? 'opacity-50' : ''">
      <label
        v-if="label"
        :for="id"
        class="block px-3.5 pb-0.5 pt-2 text-xs font-semibold text-ink-muted">
        {{ label }}
      </label>
      <div class="relative flex items-center" :class="label ? 'h-9' : 'h-10'">
        <select
          :id="id"
          :value="modelValue"
          :disabled="disabled"
          class="h-full w-full cursor-pointer appearance-none bg-transparent pl-3.5 pr-9 text-sm text-ink-strong outline-none disabled:cursor-default"
          @change="onChange">
          <option
            v-for="option in options"
            :key="option.value"
            :value="option.value">
            {{ option.label }}
          </option>
        </select>
        <ChevronDownIcon
          class="pointer-events-none absolute right-3.5 h-4 w-4 shrink-0 text-ink-faint" />
      </div>
    </div>
  </div>
</template>
