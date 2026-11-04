<script setup lang="ts">
/**
 * DbRadioCard — one choice in a list of cards, with a title and a line of
 * explanation. Group several under one `name`; the parent owns which is
 * checked. Presentation-only.
 */
withDefaults(
  defineProps<{
    name: string;
    value: string;
    checked: boolean;
    label: string;
    description?: string;
    disabled?: boolean;
  }>(),
  { description: undefined, disabled: false },
);

const emit = defineEmits<{ select: [value: string] }>();
</script>

<template>
  <label
    class="flex cursor-pointer items-start gap-3 rounded-control border px-3.5 py-3 transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-accent"
    :class="[
      checked
        ? 'border-accent bg-accent-soft'
        : 'border-line bg-panel hover:border-line-strong',
      disabled ? 'cursor-not-allowed opacity-45 hover:border-line' : '',
    ]">
    <input
      type="radio"
      class="sr-only"
      :name="name"
      :value="value"
      :checked="checked"
      :disabled="disabled"
      @change="emit('select', value)" />
    <span
      class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full border-2"
      :class="checked ? 'border-accent' : 'border-line-strong'"
      aria-hidden="true">
      <span v-if="checked" class="h-1.5 w-1.5 rounded-full bg-accent" />
    </span>
    <span class="min-w-0">
      <span class="block text-sm font-medium text-ink-strong">{{ label }}</span>
      <span v-if="description" class="block text-sm text-ink-muted">
        {{ description }}
      </span>
    </span>
  </label>
</template>
