import { createRouter, createWebHistory } from "vue-router";
import { installRouterGuards } from "./router.guards";

/**
 * Route table for the Admin UI.
 *
 * Selection state lives in the URL: a project is addressed by its unique
 * name (`/projects/p/:projectRef`) and an environment by its immutable id
 * (`/projects/p/:projectRef/e/:environmentId`). No global "active
 * environment" is stored anywhere else.
 */
export const router = createRouter({
  history: createWebHistory(),
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
      path: "/projects/p/:projectRef",
      name: "project",
      component: () => import("~/pages/Projects/Projects.page.vue"),
    },
    {
      path: "/projects/p/:projectRef/access",
      name: "project-access",
      component: () => import("~/pages/ProjectAccess/ProjectAccess.page.vue"),
    },
    {
      path: "/projects/p/:projectRef/e/:environmentId",
      name: "environment",
      component: () => import("~/pages/Projects/Projects.page.vue"),
    },
    {
      path: "/projects/p/:projectRef/e/:environmentId/tokens",
      name: "environment-tokens",
      component: () => import("~/pages/Projects/Projects.page.vue"),
    },
    {
      path: "/projects/p/:projectRef/e/:environmentId/import",
      name: "environment-import",
      component: () => import("~/pages/Projects/ImportSecrets.page.vue"),
    },
    {
      path: "/backups",
      name: "backups",
      component: () => import("~/pages/Backups/Backups.page.vue"),
      meta: { rootOnly: true },
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
      meta: { rootOnly: true },
    },
    {
      path: "/audit",
      name: "audit",
      component: () => import("~/pages/Audit/Audit.page.vue"),
    },
    {
      path: "/instance",
      name: "instance",
      component: () => import("~/pages/Instance/Instance.page.vue"),
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
      meta: { capabilities: ["users.view"] },
    },
    {
      path: "/roles",
      name: "roles",
      component: () => import("~/pages/Roles/Roles.page.vue"),
      meta: { capabilities: ["roles.manage"] },
    },
    {
      path: "/groups",
      name: "groups",
      component: () => import("~/pages/Groups/Groups.page.vue"),
      meta: { capabilities: ["users.view", "groups.manage"] },
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
  ],
});

installRouterGuards(router);
