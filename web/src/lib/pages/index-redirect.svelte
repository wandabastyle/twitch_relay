<script lang="ts">
  import { navigate } from '$lib/router/router.svelte';
  import { onMount } from 'svelte';

  onMount(() => {
    // Redirect to Twitch as default
    navigate('/twitch');
  });
</script>

<!-- This page redirects to /twitch -->
