<script setup lang="ts">
/**
 * DbSwitch — PocketBase-style toggle (41×24 rounded track, floating
 * white knob) with the Dopbase purple active state. A visually-hidden
 * native checkbox keeps keyboard, focus, and a11y behavior; the track
 * and knob are presentation bound to the model value.
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
      role="switch"
      class="peer sr-only"
      :checked="modelValue"
      :disabled="disabled"
      :aria-label="label ? undefined : label"
      @change="onChange" />
    <!-- Track: 41×24px, 7px radius (PocketBase switch geometry). -->
    <span
      aria-hidden="true"
      class="relative inline-flex h-6 w-[41px] shrink-0 rounded-[7px] transition-colors duration-150 peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-accent"
      :class="modelValue ? 'bg-accent' : 'bg-line'">
      <!-- Knob: 19×16px white pill with a soft edge shadow. -->
      <span
        class="absolute left-1 top-1 h-4 w-[19px] rounded-[5px] bg-white shadow-sm transition-transform duration-150"
        :class="modelValue ? 'translate-x-[14px]' : 'translate-x-0'" />
    </span>
    <span v-if="label" class="text-sm text-ink">
      {{ label }}
    </span>
  </label>
</template>
