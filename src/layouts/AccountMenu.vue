<script setup lang="ts">
import { onBeforeUnmount, ref, useId, watch } from "vue";
import {
  ChevronDownIcon,
  ExternalLinkIcon,
  KeyIcon,
  LogOutIcon,
  UserIcon,
} from "~/assets/icons";
import { DbSpinner } from "~/components/ui";

/**
 * AccountMenu — the sidebar footer. Shows the signed-in email and role
 * name, without an avatar, and opens links to Account, Personal tokens and the docs, plus
 * Sign out. It renders what it's given: the layout controller decides
 * whether Personal tokens shows. Closes on Escape, an outside click, or
 * a chosen link.
 */
defineProps<{
  email: string;
  roleName: string;
  showPersonalTokens: boolean;
  loggingOut: boolean;
}>();

const emit = defineEmits<{ logout: [] }>();

const DOCS_URL = "https://docs.dopbase.com";

const menuId = useId();
const open = ref(false);
const root = ref<HTMLElement | null>(null);
const trigger = ref<HTMLButtonElement | null>(null);

function close(): void {
  open.value = false;
}

function onPointerDown(event: MouseEvent): void {
  if (!root.value?.contains(event.target as Node)) close();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;
  close();
  trigger.value?.focus();
}

function stopListening(): void {
  document.removeEventListener("mousedown", onPointerDown);
  document.removeEventListener("keydown", onKeydown);
}

watch(open, (isOpen) => {
  if (!isOpen) return stopListening();
  document.addEventListener("mousedown", onPointerDown);
  document.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(stopListening);

const itemClass =
  "flex h-9 w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-sm font-medium leading-5 text-ink transition-colors hover:bg-raised hover:text-ink-strong disabled:cursor-not-allowed disabled:opacity-60 pointer-coarse:min-h-11";
</script>

<template>
  <div ref="root" class="relative">
    <div
      v-if="open"
      :id="menuId"
      class="absolute inset-x-0 bottom-full z-10 mb-1 flex flex-col gap-0.5 rounded-card border border-line bg-panel p-1 shadow-xl"
      data-testid="account-menu">
      <RouterLink :to="{ name: 'account' }" :class="itemClass" @click="close">
        <UserIcon class="h-4 w-4 text-ink-muted" />
        Account
      </RouterLink>
      <RouterLink
        v-if="showPersonalTokens"
        :to="{ name: 'account', hash: '#personal-tokens' }"
        :class="itemClass"
        @click="close">
        <KeyIcon class="h-4 w-4 text-ink-muted" />
        Personal tokens
      </RouterLink>
      <a
        :href="DOCS_URL"
        target="_blank"
        rel="noopener noreferrer"
        :class="itemClass"
        @click="close">
        <ExternalLinkIcon class="h-4 w-4 text-ink-muted" />
        Docs
      </a>
      <div class="my-0.5 h-px bg-line-soft" role="separator" />
      <button
        type="button"
        :class="itemClass"
        :disabled="loggingOut"
        data-testid="logout-button"
        @click="emit('logout')">
        <DbSpinner v-if="loggingOut" class="h-4 w-4 text-ink-muted" />
        <LogOutIcon v-else class="h-4 w-4 text-ink-muted" />
        {{ loggingOut ? "Signing out…" : "Sign out" }}
      </button>
    </div>

    <button
      ref="trigger"
      type="button"
      class="flex h-13 w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-left transition-colors hover:bg-raised"
      :aria-expanded="open"
      :aria-controls="open ? menuId : undefined"
      aria-label="Account menu"
      data-testid="account-menu-button"
      @click="open = !open">
      <span class="flex min-w-0 flex-1 flex-col items-start gap-0.5">
        <span
          class="w-full truncate font-mono text-sm leading-[22px] text-ink"
          :title="email">
          {{ email }}
        </span>
        <span
          class="rounded-full bg-raised px-2 py-px text-sm font-medium leading-5 text-ink">
          {{ roleName }}
        </span>
      </span>
      <ChevronDownIcon
        class="h-4 w-4 shrink-0 text-ink-muted transition-transform"
        :class="open ? 'rotate-180' : ''" />
    </button>
  </div>
</template>
