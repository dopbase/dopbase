<script setup lang="ts">
import { computed } from "vue";
import { CheckIcon, FolderIcon, PlusIcon } from "~/assets/icons";
import { DbButton, DbCopyButton } from "~/components/ui";
import type { AuthMethods } from "~/services/auth.api";

/**
 * SetupChecklist — what root sees on Projects while the server has no
 * projects: the setup steps carried over from /setup, with the first three
 * done and the first project as the one left. Renders props and reports
 * the click on New project.
 */
const props = defineProps<{
  /** The signed-in account, for the root step. */
  email: string;
  /** Null until loaded, or when the server didn't answer. */
  signInMethods: AuthMethods | null;
}>();

const emit = defineEmits<{
  "new-project": [];
}>();

const signInSentence = computed(() => {
  const sso = props.signInMethods?.sso;
  if (!sso) return "Passwords for now.";
  return props.signInMethods?.password
    ? `Passwords and ${sso.name}.`
    : `${sso.name} only.`;
});

const commands = ["dopbase login", "dopbase project create payments"];
</script>

<template>
  <section
    class="flex max-w-[720px] flex-col gap-7 py-4"
    aria-labelledby="setup-checklist-title"
    data-testid="setup-checklist">
    <header class="flex flex-col gap-1.5">
      <h2
        id="setup-checklist-title"
        class="text-lg font-semibold leading-[26px] text-ink-strong">
        Finish setting up Dopbase
      </h2>
      <p class="text-[15px] leading-6 text-ink-muted">
        One step left. This list goes away once your first project exists.
      </p>
      <div class="mt-1.5 flex items-center gap-3">
        <div
          class="h-1 max-w-[600px] flex-1 overflow-hidden rounded-full bg-line"
          role="progressbar"
          aria-label="Setup progress"
          aria-valuemin="0"
          aria-valuemax="4"
          aria-valuenow="3">
          <div class="h-full w-3/4 rounded-full bg-ok" />
        </div>
        <span class="shrink-0 text-sm leading-[22px] text-ink-muted">
          3 of 4 done
        </span>
      </div>
    </header>

    <ol class="flex flex-col">
      <li class="relative flex gap-4 pb-6">
        <span
          class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-ok/40"
          aria-hidden="true" />
        <span
          class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
          aria-hidden="true">
          <CheckIcon class="h-4 w-4" />
        </span>
        <div class="min-w-0">
          <p class="text-[15px] font-semibold leading-5 text-ink-strong">
            Root account
            <span class="sr-only">(done)</span>
          </p>
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            Signed in as
            <span class="font-mono [overflow-wrap:anywhere]">{{ email }}</span
            >.
          </p>
        </div>
      </li>

      <li class="relative flex gap-4 pb-6">
        <span
          class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-ok/40"
          aria-hidden="true" />
        <span
          class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
          aria-hidden="true">
          <CheckIcon class="h-4 w-4" />
        </span>
        <div class="min-w-0">
          <p class="text-[15px] font-semibold leading-5 text-ink-strong">
            Recovery kit
            <span class="sr-only">(done)</span>
          </p>
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            You can download it again from
            <RouterLink
              :to="{ name: 'status' }"
              class="text-ink underline underline-offset-2 hover:text-ink-strong">
              Status</RouterLink
            >.
          </p>
        </div>
      </li>

      <li class="relative flex gap-4 pb-6">
        <span
          class="absolute bottom-0 left-[13px] top-8 w-[1.5px] bg-line"
          aria-hidden="true" />
        <span
          class="relative flex h-7 w-7 shrink-0 items-center justify-center rounded-control bg-ok/15 text-ok"
          aria-hidden="true">
          <CheckIcon class="h-4 w-4" />
        </span>
        <div class="min-w-0">
          <p class="text-[15px] font-semibold leading-5 text-ink-strong">
            Sign-in for your team
            <span class="sr-only">(done)</span>
          </p>
          <p
            class="mt-1 text-sm leading-[22px] text-ink-muted"
            data-testid="setup-sign-in">
            {{ signInSentence }}
            <RouterLink
              v-if="!signInMethods?.sso"
              :to="{ name: 'sso' }"
              class="text-ink underline underline-offset-2 hover:text-ink-strong">
              Add single sign-on
            </RouterLink>
          </p>
        </div>
      </li>

      <li class="flex gap-4" aria-current="step">
        <span
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-control border-[1.5px] border-ink-strong text-sm font-semibold text-ink-strong"
          aria-hidden="true">
          4
        </span>
        <div class="flex min-w-0 flex-1 flex-col gap-2.5">
          <div>
            <p class="text-[15px] font-semibold leading-5 text-ink-strong">
              Create your first project
            </p>
            <p class="mt-1 text-sm leading-[22px] text-ink-muted">
              One project per app or service. Next you'll import its .env and
              run the app with it.
            </p>
          </div>
          <div
            class="flex flex-col items-center gap-2 rounded-card border border-dashed border-line bg-panel px-6 py-6 text-center">
            <FolderIcon class="h-4 w-4 text-ink-muted" />
            <p class="text-sm font-medium leading-5 text-ink-strong">
              Name it after the app, like
              <span class="font-mono">payments</span> or
              <span class="font-mono">web</span>
            </p>
            <p class="text-sm leading-[22px] text-ink-muted">
              Add its environments (dev, staging, prod) once it exists.
            </p>
            <DbButton
              variant="primary"
              class="mt-1"
              data-testid="new-project"
              @click="emit('new-project')">
              <PlusIcon class="h-4 w-4" />
              New project
            </DbButton>
          </div>
          <p class="mt-1 text-sm leading-[22px] text-ink-muted">
            Or from your terminal:
          </p>
          <div
            class="flex flex-col gap-1.5 rounded-card border border-line-soft bg-editor px-4 py-3">
            <div
              v-for="command in commands"
              :key="command"
              class="flex items-center gap-2">
              <code
                class="min-w-0 flex-1 truncate font-mono text-sm leading-[22px] text-ink-strong"
                :title="command">
                <span class="select-none text-ink-muted">$ </span>{{ command }}
              </code>
              <DbCopyButton :value="command" label="Copy command" icon-only />
            </div>
          </div>
        </div>
      </li>
    </ol>
  </section>
</template>
