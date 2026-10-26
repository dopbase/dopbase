<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import {
  ArchiveIcon,
  FolderIcon,
  HistoryIcon,
  KeyIcon,
  ServerIcon,
  UserIcon,
  UsersIcon,
  ShieldIcon,
  SlidersHorizontalIcon,
  LogOutIcon,
} from "~/assets/icons";
import type { Capability } from "~/services/access.api";
import { DbSpinner } from "~/components/ui";

/**
 * DashboardNav — primary navigation plus the account footer (email and
 * logout). Rendered by DashboardLayout in the desktop sidebar and in the
 * navigation sheet on narrow screens, so both stay in sync.
 */
defineProps<{
  email: string;
  loggingOut: boolean;
}>();

const emit = defineEmits<{ logout: [] }>();

const route = useRoute();
const auth = useAuthStore();

const navItems = [
  {
    name: "users",
    label: "Users",
    icon: UsersIcon,
    match: (r: string) => r.startsWith("/users"),
  },
  {
    name: "groups",
    label: "Groups",
    icon: ShieldIcon,
    match: (r: string) => r.startsWith("/groups"),
  },
  {
    name: "roles",
    label: "Roles",
    icon: SlidersHorizontalIcon,
    match: (r: string) => r.startsWith("/roles"),
  },
  {
    name: "projects",
    label: "Projects",
    icon: FolderIcon,
    match: (r: string) => r.startsWith("/projects"),
  },
  {
    name: "backups",
    label: "Backups",
    icon: ArchiveIcon,
    match: (r: string) => r.startsWith("/backups"),
  },
  {
    name: "sso",
    label: "Single sign-on",
    icon: KeyIcon,
    match: (r: string) => r.startsWith("/sso"),
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
/** The capabilities that show each item. Items without an entry always show. */
const needs: Partial<Record<(typeof navItems)[number]["name"], Capability[]>> =
  {
    users: ["users.view"],
    groups: ["users.view", "groups.manage"],
    backups: ["instance.backup"],
    sso: ["sso.manage"],
    roles: ["roles.manage"],
  };
const visibleNavItems = computed(() =>
  navItems.filter((item) => {
    const capabilities = needs[item.name];
    return !capabilities || auth.can(...capabilities);
  }),
);
</script>

<template>
  <nav class="flex flex-col gap-1 px-3 py-4" aria-label="Primary">
    <RouterLink
      v-for="item in visibleNavItems"
      :key="item.name"
      :to="{ name: item.name }"
      class="flex items-center gap-2.5 rounded-control px-3 py-2 text-sm transition-colors pointer-coarse:min-h-11"
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

  <div class="mt-auto border-t border-line-soft px-4 py-3.5">
    <div class="flex items-center justify-between gap-2">
      <div class="min-w-0">
        <p class="text-xs text-ink-faint">Signed in as</p>
        <p class="truncate font-mono text-xs text-ink-strong">
          {{ email }}
        </p>
      </div>
      <button
        type="button"
        class="cursor-pointer rounded-control bg-raised p-1.5 text-ink-muted transition-colors hover:bg-crit/15 hover:text-crit disabled:cursor-not-allowed disabled:opacity-60 pointer-coarse:p-3"
        :disabled="loggingOut"
        :aria-label="loggingOut ? 'Logging out...' : 'Log out'"
        :title="loggingOut ? 'Logging out...' : 'Log out'"
        data-testid="logout-button"
        @click="emit('logout')">
        <DbSpinner v-if="loggingOut" class="h-4 w-4 text-ink-muted" />
        <LogOutIcon v-else class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>
