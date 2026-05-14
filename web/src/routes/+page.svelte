<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    // Redirect to Twitch as default
    goto('/twitch');
  });
</script>

<!-- This page redirects to /twitch -->
