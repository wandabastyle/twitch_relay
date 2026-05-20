<script lang="ts">
  import { getVersion } from '$lib/api-client';
  import { onMount } from 'svelte';

  const UNKNOWN_VERSION = '…';
  const ERROR_VERSION = '?';

  let version = $state(UNKNOWN_VERSION);

  onMount(async () => {
    try {
      const { version: ver } = await getVersion();
      version = ver;
    } catch {
      version = ERROR_VERSION;
    }
  });
</script>

<p class="app-version" aria-label="App version">v{version}</p>

<style>
  .app-version {
    margin-top: 2rem;
    padding: 0.75rem 0 0;
    text-align: center;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    color: var(--muted);
    user-select: none;
  }
</style>
