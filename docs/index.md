---
layout: home

hero:
  name: "Welcome to Dopbase"
  text: "Secrets management in one binary"
  tagline: Run the server yourself or connect the same client to Dopbase Cloud. Keep projects, environments, and secrets in one place without building a platform around the platform.
  actions:
    - theme: brand
      text: Get started
      link: /guide/quick-start
    - theme: alt
      text: How Dopbase works
      link: /guide/server-client

features:
  - title: One executable
    details: The server, API, admin interface, migrations, and command-line client ship together.
  - title: Your server or ours
    details: Point the same Dopbase client at a self-hosted endpoint or Dopbase Cloud.
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
