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
  {
    type: "text",
    label: undefined,
    placeholder: undefined,
    autocomplete: undefined,
    name: undefined,
    error: undefined,
    hint: undefined,
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const id = useId();

const inputEl = ref<HTMLInputElement | null>(null);
const revealed = ref(false);

const isPassword = computed(() => props.type === "password");
const resolvedType = computed(() =>
  isPassword.value && revealed.value ? "text" : props.type,
);

/**
 * Password fields default to a masked-dots placeholder so users can
 * spot the password field even without an explicit label hint.
 */
const resolvedPlaceholder = computed(() =>
  props.placeholder ?? (isPassword.value ? "••••••••" : undefined),
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
    <!-- PocketBase-style field: the label lives inside the filled,
         borderless input and the fill lightens on focus. -->
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
      <div class="relative flex items-center" :class="label ? 'h-9' : 'h-10'">
        <input
          :id="id"
          ref="inputEl"
          :name="name"
          :type="resolvedType"
          :value="modelValue"
          :placeholder="resolvedPlaceholder"
          :autocomplete="autocomplete"
          :disabled="disabled"
          :required="required"
          :aria-invalid="error ? true : undefined"
          :aria-describedby="error ? `${id}-error` : undefined"
          class="h-full w-full bg-transparent pl-3.5 pr-3.5 text-sm text-ink-strong outline-none placeholder:text-ink-faint disabled:cursor-default"
          :class="[isPassword ? 'pr-10' : '', mono ? 'font-mono text-xs' : '']"
          @input="onInput" />
        <button
          v-if="isPassword"
          type="button"
          class="absolute right-2 top-1/2 -translate-y-1/2 cursor-pointer rounded-control p-1 text-ink-muted transition-colors hover:bg-line hover:text-ink-strong focus-visible:outline focus-visible:outline-accent"
          :aria-label="revealed ? 'Hide password' : 'Show password'"
          :aria-pressed="revealed"
          :tabindex="disabled ? -1 : 0"
          @mousedown.prevent
          @click="toggleReveal">
          <EyeOffIcon v-if="revealed" class="h-4 w-4" />
          <EyeIcon v-else class="h-4 w-4" />
        </button>
      </div>
    </div>
    <p v-if="error" :id="`${id}-error`" class="text-xs text-crit">
      {{ error }}
    </p>
    <p v-else-if="hint" class="text-xs text-ink-muted">
      {{ hint }}
    </p>
  </div>
</template>

<style scoped>
/* Match browser autofill to the field fill (mirrors PocketBase's inset
   box-shadow workaround) so autofilled fields keep the editor look. */
input:-webkit-autofill {
  box-shadow: 0 0 0 50px var(--color-raised) inset;
  -webkit-text-fill-color: var(--color-ink-strong);
  transition: background-color 9999s ease-out;
}
</style>
