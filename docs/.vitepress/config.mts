import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { defineConfig, type HeadConfig } from "vitepress";
import { assetFileNames } from "../../config/asset-file-names";

const siteUrl = "https://docs.dopbase.com";
const siteName = "Dopbase";
const siteDescription =
  "Open-source secrets manager in a single file. Self-host projects, environments, and encrypted secrets, then inject them into any process.";
const ogImage = `${siteUrl}/og-image.jpg`;

function buildJsonLd(pageData: {
  relativePath: string;
  title: string;
}): string {
  const website = {
    "@type": "WebSite",
    "@id": `${siteUrl}/#website`,
    url: siteUrl,
    name: siteName,
    description: siteDescription,
    inLanguage: "en-US",
  };

  if (pageData.relativePath === "index.md") {
    return JSON.stringify({
      "@context": "https://schema.org",
      "@graph": [
        website,
        {
          "@type": "SoftwareApplication",
          name: siteName,
          applicationCategory: "DeveloperApplication",
          operatingSystem: "Linux, macOS, Windows",
          description: siteDescription,
          url: siteUrl,
          offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
          license: "https://spdx.org/licenses/Apache-2.0.html",
        },
      ],
    });
  }

  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: pageData.title,
    description: siteDescription,
    url: siteUrl,
    inLanguage: "en-US",
    isPartOf: { "@id": `${siteUrl}/#website` },
    publisher: { "@type": "Organization", name: siteName, url: siteUrl },
  });
}

export default defineConfig({
  title: "Dopbase",
  titleTemplate: ":title · Dopbase",
  description: siteDescription,
  lang: "en-US",
  cleanUrls: true,
  lastUpdated: true,
  sitemap: { hostname: siteUrl },
  async buildEnd({ outDir }) {
    await writeFile(
      join(outDir, "robots.txt"),
      `User-agent: *\nAllow: /\n\nSitemap: ${siteUrl}/sitemap.xml\n`,
    );
  },
  transformHead({ pageData }) {
    const cleanPath = pageData.relativePath
      .replace(/(^|\/)index\.md$/, "$1")
      .replace(/\.md$/, "");
    const url = `${siteUrl}/${cleanPath}`;
    const title = pageData.title || siteName;
    const description =
      (pageData.frontmatter?.description as string | undefined) ??
      siteDescription;

    const head: HeadConfig[] = [
      ["link", { rel: "canonical", href: url }],
      ["meta", { property: "og:title", content: title }],
      ["meta", { property: "og:description", content: description }],
      ["meta", { property: "og:url", content: url }],
      [
        "meta",
        {
          property: "og:type",
          content: pageData.relativePath === "index.md" ? "website" : "article",
        },
      ],
      ["meta", { property: "og:site_name", content: siteName }],
      ["meta", { property: "og:image", content: ogImage }],
      ["meta", { property: "og:locale", content: "en-US" }],
      ["meta", { name: "twitter:card", content: "summary_large_image" }],
      ["meta", { name: "twitter:title", content: title }],
      ["meta", { name: "twitter:description", content: description }],
      ["meta", { name: "twitter:image", content: ogImage }],
      ["script", { type: "application/ld+json" }, buildJsonLd(pageData)],
    ];
    return head;
  },
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
      { text: "Admin UI", link: "/ui/" },
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
            { text: "Client configuration", link: "/cli/configuration" },
            {
              text: "Projects and environments",
              link: "/cli/environment-targeting",
            },
            { text: "Command reference", link: "/cli/commands" },
          ],
        },
      ],
      "/ui/": [
        {
          text: "Admin UI",
          items: [
            { text: "Overview", link: "/ui/" },
            { text: "Setup and sign in", link: "/ui/setup-and-sign-in" },
            {
              text: "Projects and environments",
              link: "/ui/projects-environments",
            },
            { text: "Managing secrets", link: "/ui/managing-secrets" },
            { text: "Import and export", link: "/ui/import-export" },
            { text: "Audit and instance status", link: "/ui/audit-instance" },
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
      message: "Secrets manager in a single file.",
      copyright: "Dopbase documentation",
    },
  },
});
