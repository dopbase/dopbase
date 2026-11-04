<script setup lang="ts">
import { useAccountController } from "./Account.controller";
import SsoLinkCard from "./components/SsoLinkCard.vue";
import PersonalTokensCard from "./components/PersonalTokensCard.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import { DbAlert, DbButton, DbInput, DbModal, DbTabNav } from "~/components/ui";

/**
 * Account — your profile and sign-in, and the tokens your role may hold.
 * Each tab has its own address.
 */
const controller = useAccountController();
const {
  activeTab,
  tabs,
  subtitle,
  groups,
  canOpenGroups,
  passwordOpen,
  email,
  formattedRole,
  lastLoginAt,
  formattedLastLogin,
  fullLastLogin,
  needsCurrentPassword,
  currentPassword,
  newPassword,
  confirmPassword,
  fieldErrors,
  formError,
  submitting,
} = controller;
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader title="Account" :subtitle="subtitle" />

      <DbTabNav
        v-if="tabs.length > 1"
        :tabs="tabs"
        :active="activeTab"
        label="Account sections" />

      <template v-if="activeTab === 'profile'">
        <section
          class="flex flex-col gap-3.5 rounded-card border border-line bg-panel px-5 py-4"
          aria-labelledby="profile-heading"
          data-testid="account-profile">
          <h2
            id="profile-heading"
            class="text-[15px] font-semibold leading-[22px] text-ink-strong">
            Profile
          </h2>
          <dl class="flex flex-col gap-3.5 text-sm">
            <div class="flex gap-3">
              <dt class="w-28 shrink-0 text-ink-muted sm:w-32">Email</dt>
              <dd class="min-w-0 break-all font-mono text-ink-strong">
                {{ email }}
              </dd>
            </div>
            <div class="flex items-center gap-3">
              <dt class="w-28 shrink-0 text-ink-muted sm:w-32">Role</dt>
              <dd>
                <span
                  class="inline-flex rounded-control bg-raised px-2 py-px text-sm font-medium text-ink">
                  {{ formattedRole }}
                </span>
              </dd>
            </div>
            <div v-if="groups" class="flex gap-3">
              <dt class="w-28 shrink-0 text-ink-muted sm:w-32">Groups</dt>
              <dd class="min-w-0">
                <span v-if="groups.length === 0" class="text-ink-muted">
                  None
                </span>
                <template v-for="(group, index) in groups" :key="group.id">
                  <RouterLink
                    v-if="canOpenGroups"
                    :to="{ name: 'group', params: { groupId: group.id } }"
                    class="text-accent-strong hover:underline">
                    {{ group.name }}
                  </RouterLink>
                  <span v-else class="text-accent-strong">{{
                    group.name
                  }}</span>
                  <span v-if="index < groups.length - 1" class="text-ink-muted"
                    >,
                  </span>
                </template>
              </dd>
            </div>
            <div v-if="lastLoginAt" class="flex gap-3">
              <dt class="w-28 shrink-0 text-ink-muted sm:w-32">Last sign-in</dt>
              <dd class="text-ink-strong" :title="fullLastLogin">
                {{ formattedLastLogin }} · this browser
              </dd>
            </div>
          </dl>
        </section>

        <SsoLinkCard />

        <section
          class="flex flex-wrap items-center gap-3 rounded-card border border-line bg-panel px-5 py-4"
          aria-labelledby="password-heading">
          <div class="min-w-0 flex-1">
            <h2
              id="password-heading"
              class="text-[15px] font-semibold leading-[22px] text-ink-strong">
              Password
            </h2>
            <p class="text-sm text-ink-muted">
              {{
                needsCurrentPassword
                  ? "Changing it signs you out everywhere, this browser too."
                  : "You sign in with single sign-on. A password lets you in when the provider is down."
              }}
            </p>
          </div>
          <DbButton
            data-testid="open-password"
            @click="controller.openPassword">
            {{ needsCurrentPassword ? "Change password" : "Set a password" }}
          </DbButton>
        </section>
      </template>

      <PersonalTokensCard v-else />
    </div>

    <DbModal
      :open="passwordOpen"
      :title="needsCurrentPassword ? 'Change password' : 'Set a password'"
      @close="passwordOpen = false">
      <form
        id="change-password"
        class="flex flex-col gap-4"
        novalidate
        data-testid="change-password-form"
        @submit.prevent="controller.submit">
        <p class="text-sm text-ink-muted">
          {{
            needsCurrentPassword
              ? "Saving it ends every session, including this one. You'll sign in again with the new password."
              : "Saving it ends every session, including this one."
          }}
        </p>
        <DbInput
          v-if="needsCurrentPassword"
          v-model="currentPassword"
          label="Current password"
          name="currentPassword"
          type="password"
          autocomplete="current-password"
          :error="fieldErrors.currentPassword" />
        <DbInput
          v-model="newPassword"
          label="New password"
          name="newPassword"
          type="password"
          autocomplete="new-password"
          hint="12 to 128 characters."
          :error="fieldErrors.newPassword" />
        <DbInput
          v-model="confirmPassword"
          label="Confirm new password"
          name="confirmPassword"
          type="password"
          autocomplete="new-password"
          :error="fieldErrors.confirmPassword" />
        <DbAlert v-if="formError">{{ formError }}</DbAlert>
      </form>
      <template #footer>
        <DbButton
          variant="ghost"
          :disabled="submitting"
          @click="passwordOpen = false">
          Cancel
        </DbButton>
        <DbButton
          variant="primary"
          type="submit"
          form="change-password"
          :loading="submitting">
          {{ needsCurrentPassword ? "Change password" : "Set password" }}
        </DbButton>
      </template>
    </DbModal>
  </DashboardLayout>
</template>
