<script setup lang="ts">
import { computed, ref, useId } from "vue";
import { EyeIcon, EyeOffIcon } from "~/assets/icons";

/**
 * DbInput — labeled text input with inline validation and hint slots.
 * Password inputs get a built-in show/hide (eye) toggle.
 * Presentation-only; validation messages are passed in as props.
 */
const props = withDefaults(
  defineProps<{
    modelValue: string;
    label?: string;
    type?: string;
    placeholder?: string;
    autocomplete?: string;
    name?: string;
    error?: string | null;
    hint?: string | null;
    mono?: boolean;
    disabled?: boolean;
    required?: boolean;
  }>(),
  { type: "text" },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const id = useId();

const inputEl = ref<HTMLInputElement | null>(null);
const revealed = ref(false);

const isPassword = computed(() => props.type === "password");
const resolvedType = computed(() =>
  isPassword.value && revealed.value ? "text" : props.type,
);

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

/** Toggle password visibility, keeping focus and caret in the field. */
function toggleReveal(): void {
  revealed.value = !revealed.value;
  requestAnimationFrame(() => inputEl.value?.focus());
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <label v-if="label" :for="id" class="text-xs font-medium text-ink-muted">
      {{ label }}
    </label>
    <div class="relative">
      <input
        :id="id"
        ref="inputEl"
        :name="name"
        :type="resolvedType"
        :value="modelValue"
        :placeholder="placeholder"
        :autocomplete="autocomplete"
        :disabled="disabled"
        :required="required"
        :aria-invalid="error ? true : undefined"
        :aria-describedby="error ? `${id}-error` : undefined"
        class="w-full rounded-md border bg-canvas px-3 py-2 text-sm text-ink-strong outline-none transition-colors placeholder:text-ink-faint focus:border-accent disabled:opacity-50"
        :class="[
          error ? 'border-crit/60' : 'border-line',
          mono ? 'font-mono text-xs' : '',
          isPassword ? 'pr-10' : '',
        ]"
        @input="onInput" />
      <button
        v-if="isPassword"
        type="button"
        class="absolute right-1.5 top-1/2 -translate-y-1/2 cursor-pointer rounded p-1 text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong focus-visible:outline focus-visible:outline-accent"
        :aria-label="revealed ? 'Hide password' : 'Show password'"
        :aria-pressed="revealed"
        :tabindex="disabled ? -1 : 0"
        @mousedown.prevent
        @click="toggleReveal">
        <EyeOffIcon v-if="revealed" class="h-4 w-4" />
        <EyeIcon v-else class="h-4 w-4" />
      </button>
    </div>
    <p v-if="error" :id="`${id}-error`" class="text-xs text-crit">
      {{ error }}
    </p>
    <p v-else-if="hint" class="text-xs text-ink-muted">
      {{ hint }}
    </p>
  </div>
</template>
