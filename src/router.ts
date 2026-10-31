import { createRouter, createWebHistory } from "vue-router";
import { installRouterGuards } from "./router.guards";
import { redirectLegacyEnvironment } from "./router.legacy";

/**
 * Route table for the Admin UI.
 *
 * Selection state lives in the URL: a project is addressed by its unique
 * name and an environment by its name within the project
 * (`/projects/:projectRef/:environmentName/secrets`). No global "active
 * environment" is stored anywhere else. Older `/projects/p/...` addresses
 * redirect.
 *
 * Which signed-in pages an account may open comes from
 * `~/navigation/access-map`, not from route meta.
 */
export const router = createRouter({
  history: createWebHistory(),
  // The account menu links to sections of the Account page by hash. Other
  // navigations keep the scroll position, as before.
  scrollBehavior: (to) => (to.hash ? { el: to.hash } : undefined),
  routes: [
    {
      path: "/",
      redirect: { name: "projects" },
    },
    {
      path: "/login",
      name: "login",
      component: () => import("~/pages/Login/Login.page.vue"),
      meta: { public: true, guestOnly: true },
    },
    {
      path: "/login/sso",
      name: "login-sso",
      component: () => import("~/pages/Login/SsoHandoff.page.vue"),
      meta: { public: true },
    },
    {
      path: "/setup",
      name: "setup",
      component: () => import("~/pages/Setup/Setup.page.vue"),
      meta: { public: true, setupOnly: true },
    },
    {
      path: "/projects",
      name: "projects",
      component: () => import("~/pages/Projects/Projects.page.vue"),
    },
    {
      // Opens the project's first environment, or says it has none.
      path: "/projects/:projectRef",
      name: "project",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/access",
      name: "project-access",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/settings",
      name: "project-settings",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/:environmentName/secrets",
      name: "environment",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/:environmentName/tokens",
      name: "environment-tokens",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/:environmentName/compare",
      name: "environment-compare",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/:environmentName/activity",
      name: "environment-activity",
      component: () => import("~/pages/Projects/Project.page.vue"),
    },
    {
      path: "/projects/:projectRef/:environmentName/import",
      name: "environment-import",
      component: () => import("~/pages/Projects/ImportSecrets.page.vue"),
    },
    // Addresses from before environments were named in the URL.
    {
      path: "/projects/p/:projectRef",
      redirect: (to) => ({
        name: "project",
        params: { projectRef: to.params.projectRef },
      }),
    },
    {
      path: "/projects/p/:projectRef/access",
      redirect: (to) => ({
        name: "project-access",
        params: { projectRef: to.params.projectRef },
      }),
    },
    {
      path: "/projects/p/:projectRef/e/:environmentId/:tail(tokens|import)?",
      name: "legacy-environment",
      component: () => import("~/pages/NotFound/NotFound.page.vue"),
      beforeEnter: redirectLegacyEnvironment,
    },
    {
      path: "/backups",
      name: "backups",
      component: () => import("~/pages/Backups/Backups.page.vue"),
    },
    {
      path: "/sso/confirmed",
      name: "sso-confirmed",
      component: () => import("~/pages/SsoConfirmed/SsoConfirmed.page.vue"),
      meta: { public: true },
    },
    {
      path: "/sso",
      name: "sso",
      component: () => import("~/pages/Sso/Sso.page.vue"),
    },
    {
      path: "/audit",
      name: "audit",
      component: () => import("~/pages/Audit/Audit.page.vue"),
    },
    {
      path: "/status",
      name: "status",
      component: () => import("~/pages/Instance/Instance.page.vue"),
    },
    {
      path: "/instance",
      redirect: { name: "status" },
    },
    {
      path: "/account",
      name: "account",
      component: () => import("~/pages/Account/Account.page.vue"),
    },
    {
      path: "/users",
      name: "users",
      component: () => import("~/pages/Users/Users.page.vue"),
    },
    {
      path: "/roles",
      name: "roles",
      component: () => import("~/pages/Roles/Roles.page.vue"),
    },
    {
      path: "/groups",
      name: "groups",
      component: () => import("~/pages/Groups/Groups.page.vue"),
    },
    {
      path: "/cli/approve",
      name: "cli-approve",
      component: () => import("~/pages/CliApprove/CliApprove.page.vue"),
      // The page decides about the session itself: it always wants a sign-in
      // newer than the request, so the usual login redirect is not enough.
      meta: { public: true },
    },
    {
      path: "/:pathMatch(.*)*",
      name: "not-found",
      component: () => import("~/pages/NotFound/NotFound.page.vue"),
      meta: { public: true },
    },
    {
      // Only reached by name from the guard, which keeps the address the
      // viewer asked for. By path, every unknown URL matches not-found.
      path: "/:pathMatch(.*)*",
      name: "no-access",
      component: () => import("~/pages/NoAccess/NoAccess.page.vue"),
    },
  ],
});

installRouterGuards(router);
