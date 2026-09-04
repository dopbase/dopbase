<template>
  <div class="dopbase-terminal" aria-label="Dopbase server and client example">
    <div class="dopbase-terminal__bar">
      <span class="dopbase-terminal__light"></span>
      local secrets workflow
    </div>
    <pre><span class="prompt">$</span> dopbase serve
╭──────────────────────────────────────────────────────────────────╮
│  <span class="success">Dopbase</span>                                                         │
│  Secure, Simple and Private                                      │
│  Version 0.0.12                                                  │
│                                                                  │
│  <span class="quiet">Admin UI:</span>   http://localhost:8840                               │
│  <span class="quiet">API:</span>        http://localhost:8840/api/v1                        │
│  <span class="quiet">Config:</span>     /Users/venobi/.dopbase                              │
╰──────────────────────────────────────────────────────────────────╯

<span class="prompt">$</span> dopbase status
<span class="success">Server: http://localhost:8840 (default)</span>

<span class="prompt">$</span> dopbase init storefront development --from .env
<span class="success">Created storefront/development (env_01DEV...)</span>

<span class="prompt">$</span> dopbase run storefront/development -- npm start
<span class="quiet">Loaded 17 secrets into the application process.</span></pre>
  </div>
</template>
