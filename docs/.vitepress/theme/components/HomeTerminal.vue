<template>
  <div class="dopbase-terminal" aria-label="Dopbase server and client example">
    <div class="dopbase-terminal__bar">
      <span class="dopbase-terminal__light"></span>
      server and client handshake
    </div>
    <pre><span class="prompt">$</span> dopbase serve
<span class="quiet">Admin UI</span>  http://localhost:8376
<span class="quiet">API</span>       http://localhost:8376/api
<span class="success">Ready.</span>

<span class="prompt">$</span> dopbase client connect http://localhost:8376
<span class="success">Connected to http://localhost:8376</span>

<span class="prompt">$</span> dopbase run -- npm start
<span class="quiet">Loaded 17 secrets into the application process.</span></pre>
  </div>
</template>
