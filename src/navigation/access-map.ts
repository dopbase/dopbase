import type { Capability } from "~/services/access.api";

/**
 * The capability map for signed-in pages. The sidebar and the router guard
 * both read it, so the nav never offers a page that a direct link to it
 * would refuse. A page opens for anyone holding one of its capabilities;
 * an empty list, or a route not listed here, opens for every signed-in
 * account. The server still checks every request: this only decides what
 * the console offers.
 *
 * Keys are route names, and each page's path is `/<key>`. A detail route
 * under a page (`/users/:userId`) names its page in `meta.page`, and the
 * router guard checks that page.
 */
export const pages = {
  projects: { label: "Projects", capabilities: [] },
  audit: { label: "Audit log", capabilities: [] },
  users: { label: "Users", capabilities: ["users.view"] },
  agents: { label: "AI agents", capabilities: ["users.view"] },
  groups: { label: "Groups", capabilities: ["users.view", "groups.manage"] },
  roles: { label: "Roles", capabilities: ["roles.manage"] },
  status: { label: "Status", capabilities: [] },
  backups: { label: "Backups", capabilities: ["instance.backup"] },
  sso: { label: "Single sign-on", capabilities: ["sso.manage"] },
} as const satisfies Record<
  string,
  { label: string; capabilities: readonly Capability[] }
>;

export type PageName = keyof typeof pages;

/** Sidebar sections, in the order the design fixes. */
export const sidebarSections: readonly {
  label: string;
  pages: readonly PageName[];
}[] = [
  { label: "Workspace", pages: ["projects", "audit"] },
  { label: "People & access", pages: ["users", "agents", "groups", "roles"] },
  { label: "Instance", pages: ["status", "backups", "sso"] },
];

/** `auth.can`: true when the account holds any of `capabilities`. */
export type CapabilityCheck = (...capabilities: Capability[]) => boolean;

export function isPageName(name: unknown): name is PageName {
  return typeof name === "string" && Object.hasOwn(pages, name);
}

/** Whether the account may open the route called `name`. */
export function canOpen(name: unknown, can: CapabilityCheck): boolean {
  if (!isPageName(name)) return true;
  const needed: readonly Capability[] = pages[name].capabilities;
  return needed.length === 0 || can(...needed);
}

export interface SidebarSection {
  label: string;
  items: { name: PageName; label: string }[];
}

/** The sidebar for this account. A section with nothing to open is dropped. */
export function visibleSections(can: CapabilityCheck): SidebarSection[] {
  return sidebarSections
    .map((section) => ({
      label: section.label,
      items: section.pages
        .filter((name) => canOpen(name, can))
        .map((name) => ({ name, label: pages[name].label })),
    }))
    .filter((section) => section.items.length > 0);
}
