<script setup lang="ts">
import { computed } from "vue";
import { DbMenu, DbMenuItem, DbSearchField } from "~/components/ui";
import { BotIcon, ChevronDownIcon, UserIcon, UsersIcon } from "~/assets/icons";
import {
  principalKey,
  principalMeta,
  type PrincipalChoice,
} from "~/pages/Access/GiveAccess.controller";

/**
 * PrincipalPicker — the Who field on the Give access page opened from a
 * project. A button shows who is chosen and what they hold there; it opens
 * a list of groups, people and AI agents, with a search box once there are
 * more than six. Presentation-only: the page filters, and it reports the
 * key picked (`user:usr_…` or `group:grp_…`).
 */
const props = defineProps<{
  /** Everyone you can give access to. */
  choices: PrincipalChoice[];
  /** The ones that match `filter`. */
  visible: PrincipalChoice[];
  modelValue: string;
  /** Only AI agents can be picked, because the server has RBAC off. */
  agentsOnly?: boolean;
}>();

const emit = defineEmits<{ "update:modelValue": [key: string] }>();

const filter = defineModel<string>("filter", { required: true });

const chosen = computed(
  () =>
    props.choices.find((choice) => principalKey(choice) === props.modelValue) ??
    null,
);

const SECTIONS = [
  { key: "groups", label: "Groups", icon: UsersIcon },
  { key: "people", label: "People", icon: UserIcon },
  { key: "agents", label: "AI agents", icon: BotIcon },
] as const;

const sections = computed(() =>
  SECTIONS.map((section) => ({
    ...section,
    choices: props.visible.filter((choice) => choice.group === section.key),
  })).filter((section) => section.choices.length > 0),
);

const chosenIcon = computed(
  () =>
    SECTIONS.find((section) => section.key === chosen.value?.group)?.icon ??
    UserIcon,
);

function pick(key: string): void {
  filter.value = "";
  emit("update:modelValue", key);
}
</script>

<template>
  <DbMenu align="start" match-width>
    <template #trigger="{ toggle, attrs, open }">
      <button
        type="button"
        class="flex min-h-14 w-full cursor-pointer items-center gap-3 rounded-control border bg-raised px-3.5 py-2.5 text-left transition-colors hover:border-line-strong"
        :class="open ? 'border-accent/60' : 'border-line'"
        v-bind="attrs"
        aria-label="Who"
        data-testid="principal-picker"
        @click="toggle">
        <component :is="chosenIcon" class="h-4 w-4 shrink-0 text-ink-muted" />
        <span v-if="chosen" class="flex min-w-0 flex-1 flex-col">
          <span class="truncate font-mono text-sm font-medium text-ink-strong">
            {{ chosen.label }}
          </span>
          <span class="truncate text-[13px] text-ink-muted">
            {{ principalMeta(chosen) }}
          </span>
        </span>
        <span v-else class="flex-1 text-sm text-ink-faint">
          {{
            agentsOnly
              ? "Choose an AI agent"
              : "Choose a person, AI agent or group"
          }}
        </span>
        <ChevronDownIcon class="h-4 w-4 shrink-0 text-ink-faint" />
      </button>
    </template>
    <div v-if="choices.length > 6" class="p-1 pb-1.5">
      <DbSearchField
        v-model="filter"
        :placeholder="
          agentsOnly
            ? 'Search AI agents'
            : 'Search people, AI agents and groups'
        "
        data-testid="principal-search" />
    </div>
    <template v-for="section in sections" :key="section.key">
      <p class="px-2.5 pb-1 pt-2 text-[13px] text-ink-faint">
        {{ section.label }}
      </p>
      <DbMenuItem
        v-for="choice in section.choices"
        :key="principalKey(choice)"
        :selected="principalKey(choice) === modelValue"
        data-testid="principal-choice"
        @select="pick(principalKey(choice))">
        <template #icon
          ><component :is="section.icon" class="h-4 w-4"
        /></template>
        <span :class="choice.group === 'groups' ? '' : 'font-mono'">
          {{ choice.label }}
        </span>
        <template #trailing>
          <span class="shrink-0 text-[13px] font-normal text-ink-faint">
            {{ principalMeta(choice) }}
          </span>
        </template>
      </DbMenuItem>
    </template>
    <p
      v-if="visible.length === 0"
      class="px-2.5 py-3 text-center text-sm text-ink-muted">
      {{
        choices.length === 0
          ? "There's nobody you can list."
          : `Nobody matches "${filter.trim()}".`
      }}
    </p>
  </DbMenu>
</template>
