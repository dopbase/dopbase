<script setup lang="ts">
/**
 * DbTextarea — labeled multiline input for secret values.
 * Presentation-only.
 */
withDefaults(
  defineProps<{
    modelValue: string;
    label?: string;
    placeholder?: string;
    name?: string;
    rows?: number;
    error?: string | null;
    hint?: string | null;
    disabled?: boolean;
    required?: boolean;
  }>(),
  { rows: 4 },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const id = `db-textarea-${Math.random().toString(36).slice(2)}`;

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLTextAreaElement).value);
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <label v-if="label" :for="id" class="text-xs font-medium text-ink-muted">
      {{ label }}
    </label>
    <textarea
      :id="id"
      :name="name"
      :value="modelValue"
      :rows="rows"
      :placeholder="placeholder"
      :disabled="disabled"
      :required="required"
      :aria-invalid="error ? true : undefined"
      class="w-full resize-y rounded-md border bg-canvas px-3 py-2 font-mono text-xs leading-relaxed text-ink-strong outline-none transition-colors placeholder:text-ink-faint focus:border-accent disabled:opacity-50"
      :class="error ? 'border-crit/60' : 'border-line'"
      @input="onInput" />
    <p v-if="error" class="text-xs text-crit">
      {{ error }}
    </p>
    <p v-else-if="hint" class="text-xs text-ink-muted">
      {{ hint }}
    </p>
  </div>
</template>
