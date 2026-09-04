import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { defineConfig, type HeadConfig } from "vitepress";
import { assetFileNames } from "../../config/asset-file-names.ts";

const siteUrl = "https://docs.dopbase.com";
const siteName = "Dopbase";
const projectUrl = "https://github.com/dopbase/dopbase";
const siteDescription =
  "Dopbase is an open-source secrets manager for developers. Self-host one executable, organize secrets by environment, and inject them into applications.";
const ogImage = `${siteUrl}/og-image.jpg`;
const ogImageAlt =
  "Dopbase documentation for the CLI, Admin UI, API, and self-hosting";

function buildJsonLd(pageData: {
  relativePath: string;
  title: string;
  description: string;
  url: string;
}): string {
  const project = {
    "@type": "Organization",
    name: "Dopbase project",
    url: "https://dopbase.com",
    sameAs: projectUrl,
  };
  const website = {
    "@type": "WebSite",
    "@id": `${siteUrl}/#website`,
    url: `${siteUrl}/`,
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
          operatingSystem: "Linux, macOS",
          description: pageData.description,
          url: pageData.url,
          offers: { "@type": "Offer", price: "0", priceCurrency: "USD" },
          license: "https://spdx.org/licenses/Apache-2.0.html",
          author: project,
        },
      ],
    });
  }

  return JSON.stringify({
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: pageData.title,
    description: pageData.description,
    url: pageData.url,
    mainEntityOfPage: pageData.url,
    inLanguage: "en-US",
    isPartOf: { "@id": `${siteUrl}/#website` },
    author: project,
    publisher: project,
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
      ["meta", { property: "og:image:type", content: "image/jpeg" }],
      ["meta", { property: "og:image:width", content: "1200" }],
      ["meta", { property: "og:image:height", content: "630" }],
      ["meta", { property: "og:image:alt", content: ogImageAlt }],
      ["meta", { property: "og:locale", content: "en-US" }],
      ["meta", { name: "twitter:card", content: "summary_large_image" }],
      ["meta", { name: "twitter:title", content: title }],
      ["meta", { name: "twitter:description", content: description }],
      ["meta", { name: "twitter:image", content: ogImage }],
      ["meta", { name: "twitter:image:alt", content: ogImageAlt }],
      [
        "script",
        { type: "application/ld+json" },
        buildJsonLd({
          relativePath: pageData.relativePath,
          title,
          description,
          url,
        }),
      ],
    ];
    return head;
  },
  head: [
    [
      "link",
      {
        rel: "icon",
        type: "image/svg+xml",
        sizes: "any",
        href: "/favicon.svg",
      },
    ],
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
    logo: "/logo.svg",
    siteTitle: "Dopbase",
    search: { provider: "local" },
    socialLinks: [{ icon: "github", link: projectUrl }],
    editLink: {
      pattern: `${projectUrl}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },
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
      message: "Open-source secrets management for developers.",
      copyright: "Documentation maintained by the Dopbase project",
    },
  },
});
