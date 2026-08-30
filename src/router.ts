import { createRouter, createWebHistory } from "vue-router";
import { installRouterGuards } from "./router.guards";

/**
 * Route table for the Admin UI.
 *
 * Selection state lives in the URL: a project is addressed by its unique
 * name (`/workspace/p/:projectRef`) and an environment by its immutable id
 * (`/workspace/p/:projectRef/e/:environmentId`). No global "active
 * environment" is stored anywhere else.
 */
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: { name: "workspace" },
    },
    {
      path: "/login",
      name: "login",
      component: () => import("~/pages/Login/Login.page.vue"),
      meta: { public: true, guestOnly: true },
    },
    {
      path: "/setup",
      name: "setup",
      component: () => import("~/pages/Setup/Setup.page.vue"),
      meta: { public: true, setupOnly: true },
    },
    {
      path: "/workspace",
      name: "workspace",
      component: () => import("~/pages/Workspace/Workspace.page.vue"),
    },
    {
      path: "/workspace/p/:projectRef",
      name: "project",
      component: () => import("~/pages/Workspace/Workspace.page.vue"),
    },
    {
      path: "/workspace/p/:projectRef/e/:environmentId",
      name: "environment",
      component: () => import("~/pages/Workspace/Workspace.page.vue"),
    },
    {
      path: "/workspace/p/:projectRef/e/:environmentId/tokens",
      name: "environment-tokens",
      component: () => import("~/pages/Workspace/Workspace.page.vue"),
    },
    {
      path: "/workspace/p/:projectRef/e/:environmentId/import",
      name: "environment-import",
      component: () => import("~/pages/Workspace/ImportSecrets.page.vue"),
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
      path: "/:pathMatch(.*)*",
      name: "not-found",
      component: () => import("~/pages/NotFound/NotFound.page.vue"),
      meta: { public: true },
    },
  ],
});

installRouterGuards(router);
