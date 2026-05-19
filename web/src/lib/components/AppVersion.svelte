<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '$lib/api-client';

  let version = $state('…');

  onMount(async () => {
    try {
      const payload = await getVersion();
      version = payload.version;
    } catch {
      version = '?';
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
