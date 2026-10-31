<script setup lang="ts">
import { AlertTriangleIcon, DopbaseIcon, MenuIcon } from "~/assets/icons";
import { DbButton, DbModal, DbSheet } from "~/components/ui";
import ReauthModal from "~/components/app/ReauthModal.vue";
import DashboardNav from "./DashboardNav.vue";
import { useDashboardLayoutController } from "./DashboardLayout.controller";

/**
 * DashboardLayout — the authenticated app shell.
 *
 * From `lg` up: a fixed sidebar with the brand, the grouped navigation and
 * the account menu beside the content. Below `lg` the sidebar folds into a 48px top
 * bar whose menu button opens the same navigation in a sheet. The global
 * reauthentication dialog lives here so every screen inherits it.
 */
const {
  email,
  roleName,
  sections,
  activePage,
  showPersonalTokens,
  sessionExpired,
  loggingOut,
  navOpen,
  logout,
  signInAgain,
} = useDashboardLayoutController();
</script>

<template>
  <div class="flex min-h-svh flex-col lg:flex-row">
    <!-- Narrow screens: top bar with the navigation behind a menu button -->
    <header
      class="sticky top-0 z-30 flex h-12 shrink-0 items-center gap-2.5 border-b border-line bg-panel pl-4 pr-1 lg:hidden">
      <DopbaseIcon class="h-6 w-6 shrink-0" />
      <p class="font-mono text-sm font-semibold text-ink-strong">Dopbase</p>
      <button
        type="button"
        class="ml-auto flex h-11 w-11 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
        aria-label="Open navigation"
        :aria-expanded="navOpen"
        data-testid="nav-open"
        @click="navOpen = true">
        <MenuIcon class="h-5 w-5" />
      </button>
    </header>

    <DbSheet :open="navOpen" title="Dopbase" @close="navOpen = false">
      <DashboardNav
        :sections="sections"
        :active-page="activePage"
        :email="email"
        :role-name="roleName"
        :show-personal-tokens="showPersonalTokens"
        :logging-out="loggingOut"
        @logout="logout" />
    </DbSheet>

    <!-- Wide screens: fixed sidebar -->
    <aside
      class="sticky top-0 hidden h-svh w-60 shrink-0 flex-col border-r border-line-soft bg-panel lg:flex">
      <div class="flex items-center gap-2.5 px-4 pb-3 pt-4">
        <span class="flex h-8 w-8 shrink-0 items-center justify-center">
          <DopbaseIcon class="h-7.5 w-7.5" />
        </span>
        <div class="min-w-0">
          <p
            class="text-[15px] font-semibold leading-5 tracking-[-0.3px] text-ink-strong">
            Dopbase
          </p>
          <p
            class="truncate font-mono text-sm leading-5.5 text-ink-faint"
            title="Admin Console">
            Admin Console
          </p>
        </div>
      </div>
      <DashboardNav
        :sections="sections"
        :active-page="activePage"
        :email="email"
        :role-name="roleName"
        :show-personal-tokens="showPersonalTokens"
        :logging-out="loggingOut"
        @logout="logout" />
    </aside>

    <main class="min-w-0 flex-1">
      <slot />
    </main>

    <ReauthModal />

    <DbModal
      :open="sessionExpired"
      title="Session expired"
      size="sm"
      layer="session"
      persistent>
      <div class="flex flex-col gap-4">
        <div class="flex items-start gap-3">
          <div
            class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-warn/30 bg-warn/10 text-warn">
            <AlertTriangleIcon class="h-4 w-4" />
          </div>
          <p class="text-sm text-ink">
            Your session has expired. Sign in again to continue.
          </p>
        </div>
        <div class="flex justify-end">
          <DbButton variant="primary" @click="signInAgain">
            Sign in again
          </DbButton>
        </div>
      </div>
    </DbModal>
  </div>
</template>
