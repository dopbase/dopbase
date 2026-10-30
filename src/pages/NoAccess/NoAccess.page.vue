<script setup lang="ts">
import { useNoAccessController } from "./NoAccess.controller";
import { DashboardLayout } from "~/layouts";
import { DbButton, DbEmptyState } from "~/components/ui";
import { LockIcon } from "~/assets/icons";

/**
 * NoAccess — shown at the address of a page the viewer's role can't open,
 * so a shared link explains what's missing instead of landing on Projects.
 */
const { title, description } = useNoAccessController();
</script>

<template>
  <DashboardLayout>
    <div
      class="flex min-h-[calc(100svh-3rem)] items-center px-4 py-10 lg:min-h-svh">
      <DbEmptyState
        class="mx-auto w-full max-w-[520px]"
        :title="title"
        :description="description"
        data-testid="no-access">
        <template #icon>
          <LockIcon class="h-[18px] w-[18px]" />
        </template>
        <template #actions>
          <RouterLink :to="{ name: 'projects' }">
            <DbButton variant="primary">Back to projects</DbButton>
          </RouterLink>
        </template>
      </DbEmptyState>
    </div>
  </DashboardLayout>
</template>
