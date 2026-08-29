---
layout: home

hero:
  name: "Welcome to Dopbase"
  text: "Secrets manager in a single file"
  tagline: Run the server yourself. Keep projects, environments, and secrets in one place without building a platform around the platform.
  actions:
    - theme: brand
      text: Get started
      link: /guide/quick-start
    - theme: alt
      text: How Dopbase works
      link: /guide/server-client

features:
  - title: One file to install
    details: The server, API, admin interface, migrations, and command-line client ship together.
  - title: Built for self-hosting
    details: Run Dopbase on your own infrastructure with SQLite storage and a separate master key.
  - title: Built around applications
    details: Organize secrets by project and environment, then inject them directly into a process.
  - title: Secure by design
    details: Keep encrypted data separate from master key material and never put secret values in logs.
---

<div class="dopbase-home">
  <HomeTerminal />

  <div class="dopbase-path" aria-label="Dopbase workflow">
    <div><strong>01 / Serve</strong><span>Start Dopbase or use Cloud.</span></div>
    <div><strong>02 / Connect</strong><span>Choose the active server.</span></div>
    <div><strong>03 / Manage</strong><span>Store secrets by environment.</span></div>
    <div><strong>04 / Run</strong><span>Inject them without a .env file.</span></div>
  </div>
</div>
