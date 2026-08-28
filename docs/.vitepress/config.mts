import { defineConfig } from "vitepress";
import { assetFileNames } from "../../config/asset-file-names.ts";

export default defineConfig({
  title: "Dopbase",
  description: "Open-source secrets management in one binary.",
  lang: "en-US",
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" }],
    ["meta", { name: "theme-color", content: "#863bff" }],
  ],
  markdown: {
    theme: { light: "github-light", dark: "github-dark" },
  },
  vite: {
    publicDir: "../public",
    build: {
      rolldownOptions: {
        output: {
          assetFileNames,
        },
      },
    },
  },
  themeConfig: {
    logo: "/favicon.svg",
    siteTitle: "Dopbase",
    search: { provider: "local" },
    nav: [
      { text: "Guide", link: "/guide/" },
      { text: "CLI", link: "/cli/" },
      { text: "Self-hosting", link: "/self-hosting/" },
      { text: "Cloud", link: "/cloud/" },
      {
        text: "Reference",
        items: [
          { text: "Security", link: "/reference/security" },
          { text: "Audit events", link: "/reference/audit-events" },
          { text: "REST API", link: "/reference/api" },
          { text: "Glossary", link: "/reference/glossary" },
        ],
      },
      { text: "About", link: "/about/" },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Start here",
          items: [
            { text: "Introduction", link: "/guide/" },
            { text: "Product status", link: "/guide/product-status" },
            { text: "Quick start", link: "/guide/quick-start" },
          ],
        },
        {
          text: "Learn Dopbase",
          items: [
            {
              text: "Projects and environments",
              link: "/guide/projects-environments-secrets",
            },
            { text: "Server and client", link: "/guide/server-client" },
            { text: "Import a .env file", link: "/guide/import-env" },
            { text: "Run an application", link: "/guide/run-an-application" },
          ],
        },
      ],
      "/cli/": [
        {
          text: "Command line",
          items: [
            { text: "CLI overview", link: "/cli/" },
            { text: "serve", link: "/cli/serve" },
            { text: "client connect", link: "/cli/client-connect" },
            { text: "Command reference", link: "/cli/commands" },
          ],
        },
      ],
      "/self-hosting/": [
        {
          text: "Self-hosting",
          items: [
            { text: "Overview", link: "/self-hosting/" },
            {
              text: "Storage and backups",
              link: "/self-hosting/storage-backups",
            },
            { text: "Encryption keys", link: "/self-hosting/encryption-keys" },
            { text: "Operations", link: "/self-hosting/operations" },
          ],
        },
      ],
      "/cloud/": [
        {
          text: "Dopbase Cloud",
          items: [
            { text: "Overview", link: "/cloud/" },
            { text: "Connect to Cloud", link: "/cloud/connect" },
            {
              text: "Choose a deployment",
              link: "/cloud/self-hosted-vs-cloud",
            },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "Security", link: "/reference/security" },
            { text: "Identity and tokens", link: "/reference/identity" },
            { text: "Audit events", link: "/reference/audit-events" },
            { text: "REST API", link: "/reference/api" },
            { text: "Troubleshooting", link: "/reference/troubleshooting" },
            { text: "Glossary", link: "/reference/glossary" },
          ],
        },
      ],
      "/about/": [
        {
          text: "About Dopbase",
          items: [
            { text: "Project principles", link: "/about/" },
            { text: "Open source", link: "/about/open-source" },
            { text: "Roadmap", link: "/about/roadmap" },
            { text: "Product boundaries", link: "/about/product-boundaries" },
          ],
        },
      ],
    },
    outline: { level: [2, 3], label: "On this page" },
    docFooter: { prev: "Previous page", next: "Next page" },
    returnToTopLabel: "Back to top",
    sidebarMenuLabel: "Menu",
    darkModeSwitchLabel: "Appearance",
    footer: {
      message: "Open-source secrets management in one binary.",
      copyright: "Dopbase documentation",
    },
  },
});
