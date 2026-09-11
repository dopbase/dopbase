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
  {
    rows: 4,
    label: undefined,
    placeholder: undefined,
    name: undefined,
    error: undefined,
    hint: undefined,
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const id = `db-textarea-${Math.random().toString(36).slice(2)}`;

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLTextAreaElement).value);
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <!-- PocketBase-style field: label inside the filled, borderless
         textarea. The fill lightens on focus. -->
    <div
      class="rounded-control bg-raised transition-colors duration-150 focus-within:bg-line"
      :class="[
        error ? 'bg-crit/20 focus-within:bg-crit/25' : '',
        disabled ? 'opacity-50' : '',
      ]">
      <label
        v-if="label"
        :for="id"
        class="block px-3.5 pb-0.5 pt-2 text-xs font-semibold text-ink-muted">
        {{ label
        }}<span v-if="required" class="ml-0.5 text-crit" aria-hidden="true"
          >*</span
        >
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
        class="w-full resize-y bg-transparent pb-2.5 pl-3.5 pr-3.5 pt-1 font-mono text-xs leading-relaxed text-ink-strong outline-none placeholder:text-ink-faint disabled:cursor-default"
        :class="!label ? 'pb-2.5 pt-2.5' : ''"
        @input="onInput" />
    </div>
    <p v-if="error" class="text-xs text-crit">
      {{ error }}
    </p>
    <p v-else-if="hint" class="text-xs text-ink-muted">
      {{ hint }}
    </p>
  </div>
</template>

<style scoped>
/* Match browser autofill to the field fill (mirrors PocketBase). */
textarea:-webkit-autofill {
  box-shadow: 0 0 0 50px var(--color-raised) inset;
  -webkit-text-fill-color: var(--color-ink-strong);
  transition: background-color 9999s ease-out;
}
</style>
