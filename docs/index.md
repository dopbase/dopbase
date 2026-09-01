---
layout: home
title: "Dopbase: open-source secrets manager for developers"
titleTemplate: false
description: Dopbase is an open-source secrets manager for developers. Self-host one executable, organize secrets by environment, and inject them into applications.

hero:
  name: "Dopbase"
  text: "Secrets management for developers"
  tagline: Self-host the server, organize secrets by project and environment, and load them directly into your applications.
  actions:
    - theme: brand
      text: Get started
      link: /guide/quick-start
    - theme: alt
      text: How Dopbase works
      link: /guide/server-client

features:
  - title: One executable
    details: Install the server, REST API, Admin UI, migrations, and command-line client together.
  - title: Self-hosted
    details: Run Dopbase on your infrastructure with SQLite storage and a separately stored master key.
  - title: Application-focused
    details: Store secrets by project and environment, then load them directly into an application process.
  - title: Explicit security controls
    details: Encrypt values before storage, separate master-key material, and exclude plaintext secrets from logs.
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
