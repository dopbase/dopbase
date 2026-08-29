<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import {
  TerminalIcon,
  FolderIcon,
  HistoryIcon,
  ServerIcon,
  UserIcon,
  LogOutIcon,
} from "~/assets/icons";
import ReauthModal from "~/components/app/ReauthModal.vue";

/**
 * DashboardLayout — the authenticated app shell.
 *
 * A fixed sidebar rail with primary navigation plus the account footer
 * (email + logout), and a content area rendered from the default slot.
 * The global reauthentication dialog lives here so every screen inherits
 * it.
 */
const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const navItems = [
  {
    name: "workspace",
    label: "Projects",
    icon: FolderIcon,
    match: (r: string) => r.startsWith("/workspace"),
  },
  {
    name: "audit",
    label: "Audit",
    icon: HistoryIcon,
    match: (r: string) => r.startsWith("/audit"),
  },
  {
    name: "instance",
    label: "Instance",
    icon: ServerIcon,
    match: (r: string) => r.startsWith("/instance"),
  },
  {
    name: "account",
    label: "Account",
    icon: UserIcon,
    match: (r: string) => r.startsWith("/account"),
  },
] as const;

const isActive = (item: (typeof navItems)[number]): boolean =>
  item.match(route.path);

const email = computed(() => auth.session?.email ?? "");

async function logout(): Promise<void> {
  await auth.logout();
  router.push({ name: "login" });
}
</script>

<template>
  <div class="flex min-h-svh">
    <aside
      class="sticky top-0 flex h-svh w-60 shrink-0 flex-col border-r border-line bg-panel">
      <!-- Brand -->
      <div
        class="flex items-center gap-2.5 border-b border-line-soft px-5 py-4">
        <div
          class="flex h-8 w-8 items-center justify-center rounded-md border border-accent/40 bg-accent-soft text-accent-strong">
          <TerminalIcon class="h-4 w-4" />
        </div>
        <div class="leading-tight">
          <p class="font-mono text-sm font-semibold text-ink-strong">dopbase</p>
          <p class="text-xs text-ink-muted">admin console</p>
        </div>
      </div>

      <!-- Primary navigation -->
      <nav class="flex flex-col gap-1 px-3 py-4">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="{ name: item.name }"
          class="flex items-center gap-2.5 rounded-md px-3 py-2 text-sm transition-colors"
          :class="
            isActive(item)
              ? 'bg-accent-soft text-ink-strong'
              : 'text-ink-muted hover:bg-raised hover:text-ink-strong'
          "
          :aria-current="isActive(item) ? 'page' : undefined">
          <component :is="item.icon" class="h-4 w-4" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <!-- Account footer -->
      <div class="mt-auto border-t border-line-soft px-4 py-3.5">
        <div class="flex items-center justify-between gap-2">
          <div class="min-w-0">
            <p class="text-xs uppercase tracking-wide text-ink-faint">
              signed in as
            </p>
            <p class="truncate font-mono text-xs text-ink-strong">
              {{ email }}
            </p>
          </div>
          <button
            type="button"
            class="cursor-pointer rounded-md border border-line bg-raised p-1.5 text-ink-muted transition-colors hover:border-crit/40 hover:text-crit"
            aria-label="Log out"
            @click="logout">
            <LogOutIcon class="h-4 w-4" />
          </button>
        </div>
      </div>
    </aside>

    <main class="min-w-0 flex-1">
      <slot />
    </main>

    <ReauthModal />
  </div>
</template>
