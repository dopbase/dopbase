<script setup lang="ts">
import type { Component } from "vue";
import {
  ArchiveIcon,
  FolderIcon,
  HistoryIcon,
  KeyIcon,
  ServerIcon,
  SlidersHorizontalIcon,
  UserIcon,
  UsersIcon,
} from "~/assets/icons";
import type { PageName, SidebarSection } from "~/navigation/access-map";
import AccountMenu from "./AccountMenu.vue";

/**
 * DashboardNav — the grouped sidebar navigation plus the account menu.
 * DashboardLayout renders it in the desktop sidebar and in the navigation
 * sheet on narrow screens, so both stay in sync. The sections arrive
 * already filtered by the capability map.
 */
defineProps<{
  sections: SidebarSection[];
  activePage: PageName | null;
  email: string;
  roleName: string;
  showPersonalTokens: boolean;
  loggingOut: boolean;
}>();

const emit = defineEmits<{ logout: [] }>();

const icons: Record<PageName, Component> = {
  projects: FolderIcon,
  audit: HistoryIcon,
  users: UserIcon,
  groups: UsersIcon,
  roles: SlidersHorizontalIcon,
  status: ServerIcon,
  backups: ArchiveIcon,
  sso: KeyIcon,
};
</script>

<template>
  <nav
    class="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto px-3 py-1"
    aria-label="Primary">
    <section
      v-for="section in sections"
      :key="section.label"
      class="flex flex-col gap-0.5 py-2"
      :aria-label="section.label">
      <p class="px-2.5 py-1 text-sm font-medium leading-5 text-ink-faint">
        {{ section.label }}
      </p>
      <RouterLink
        v-for="item in section.items"
        :key="item.name"
        :to="{ name: item.name }"
        class="group flex h-[34px] items-center gap-2.5 rounded-control px-2.5 text-sm font-medium leading-5 transition-colors pointer-coarse:min-h-11"
        :class="
          item.name === activePage
            ? 'bg-accent-soft text-ink-strong'
            : 'text-ink hover:bg-raised hover:text-ink-strong'
        "
        :aria-current="item.name === activePage ? 'page' : undefined">
        <component
          :is="icons[item.name]"
          class="h-4 w-4 shrink-0"
          :class="
            item.name === activePage
              ? 'text-accent-strong'
              : 'text-ink-muted group-hover:text-ink-strong'
          " />
        <span class="truncate">{{ item.label }}</span>
      </RouterLink>
    </section>
  </nav>

  <div class="border-t border-line-soft px-3 pb-3 pt-2">
    <AccountMenu
      :email="email"
      :role-name="roleName"
      :show-personal-tokens="showPersonalTokens"
      :logging-out="loggingOut"
      @logout="emit('logout')" />
  </div>
</template>
