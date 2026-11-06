import type { Capability, Feature } from "~/services/access.api";

/**
 * The capability map for signed-in pages. The sidebar and the router guard
 * both read it, so the nav never offers a page that a direct link to it
 * would refuse. A page opens for anyone holding one of its capabilities;
 * an empty list, or a route not listed here, opens for every signed-in
 * account. The server still checks every request: this only decides what
 * the console offers.
 *
 * A page with a `feature` opens only while the server has it on; with it
 * off the page is gone from the sidebar and a direct link says how to
 * turn it on.
 *
 * Keys are route names, and each page's path is `/<key>`. A detail route
 * under a page (`/users/:userId`) names its page in `meta.page`, and the
 * router guard checks that page.
 */
export const pages = {
  projects: { label: "Projects", capabilities: [] },
  audit: { label: "Audit log", capabilities: [] },
  users: { label: "Users", capabilities: ["users.view"] },
  agents: {
    label: "AI agents",
    capabilities: ["agents.view", "agents.view_all"],
  },
  groups: {
    label: "Groups",
    capabilities: ["users.view", "groups.manage"],
    feature: "rbac",
  },
  roles: {
    label: "Roles",
    capabilities: ["roles.manage"],
    feature: "rbac.roles",
  },
  status: { label: "Status", capabilities: [] },
  backups: { label: "Backups", capabilities: ["instance.backup"] },
  sso: {
    label: "Single sign-on",
    capabilities: ["sso.manage"],
    feature: "sso",
  },
} as const satisfies Record<
  string,
  { label: string; capabilities: readonly Capability[]; feature?: Feature }
>;

export type PageName = keyof typeof pages;

/** Sidebar sections, in the order the design fixes. */
export const sidebarSections: readonly {
  label: string;
  pages: readonly PageName[];
}[] = [
  { label: "Workspace", pages: ["projects", "audit"] },
  { label: "Access", pages: ["users", "agents", "groups", "roles"] },
  { label: "Instance", pages: ["status", "backups", "sso"] },
];

/** `auth.can`: true when the account holds any of `capabilities`. */
export type CapabilityCheck = (...capabilities: Capability[]) => boolean;

/** `auth.hasFeature`: true when the server has `feature` on. */
export type FeatureCheck = (feature: Feature) => boolean;

const allFeatures: FeatureCheck = () => true;

/** The feature a page needs, if any. */
export function pageFeature(name: unknown): Feature | null {
  if (!isPageName(name)) return null;
  const page = pages[name];
  return "feature" in page ? page.feature : null;
}

/** Whether the server has the feature the route called `name` needs. */
export function featureOn(name: unknown, has: FeatureCheck): boolean {
  const feature = pageFeature(name);
  return feature === null || has(feature);
}

export function isPageName(name: unknown): name is PageName {
  return typeof name === "string" && Object.hasOwn(pages, name);
}

/** Whether the account may open the route called `name`. */
export function canOpen(
  name: unknown,
  can: CapabilityCheck,
  has: FeatureCheck = allFeatures,
): boolean {
  if (!isPageName(name)) return true;
  if (!featureOn(name, has)) return false;
  const needed: readonly Capability[] = pages[name].capabilities;
  return needed.length === 0 || can(...needed);
}

export interface SidebarSection {
  label: string;
  items: { name: PageName; label: string }[];
}

/** The sidebar for this account. A section with nothing to open is dropped. */
export function visibleSections(
  can: CapabilityCheck,
  has: FeatureCheck = allFeatures,
): SidebarSection[] {
  return sidebarSections
    .map((section) => ({
      label: section.label,
      items: section.pages
        .filter((name) => canOpen(name, can, has))
        .map((name) => ({ name, label: pages[name].label })),
    }))
    .filter((section) => section.items.length > 0);
}
