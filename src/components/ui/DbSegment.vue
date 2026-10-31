<script setup lang="ts">
/**
 * DbSegment — a segmented switch between a few views of the same thing,
 * such as Table and .env. Behaves as a radio group: arrow keys move the
 * choice. Presentation-only.
 */
const props = defineProps<{
  modelValue: string;
  options: readonly { value: string; label: string }[];
  /** Names the group for screen readers. */
  label: string;
}>();

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

function move(step: number, event: KeyboardEvent): void {
  const index = props.options.findIndex(
    (option) => option.value === props.modelValue,
  );
  const next =
    props.options[(index + step + props.options.length) % props.options.length];
  if (!next) return;
  emit("update:modelValue", next.value);
  const group = (event.currentTarget as HTMLElement).parentElement;
  requestAnimationFrame(() =>
    group
      ?.querySelector<HTMLButtonElement>(`[data-value="${next.value}"]`)
      ?.focus(),
  );
}
</script>

<template>
  <div
    class="inline-flex shrink-0 items-center gap-0.5 rounded-control border border-line bg-panel p-0.5"
    role="radiogroup"
    :aria-label="label">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      role="radio"
      :aria-checked="option.value === modelValue"
      :tabindex="option.value === modelValue ? 0 : -1"
      :data-value="option.value"
      class="cursor-pointer whitespace-nowrap rounded-[4px] px-2.5 py-1 text-sm font-medium leading-5 transition-colors pointer-coarse:py-2"
      :class="
        option.value === modelValue
          ? 'bg-line-strong text-ink-strong'
          : 'text-ink-muted hover:text-ink'
      "
      @click="emit('update:modelValue', option.value)"
      @keydown.right.prevent="move(1, $event)"
      @keydown.down.prevent="move(1, $event)"
      @keydown.left.prevent="move(-1, $event)"
      @keydown.up.prevent="move(-1, $event)">
      {{ option.label }}
    </button>
  </div>
</template>
