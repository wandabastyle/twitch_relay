<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    isLoading: boolean;
    error: string | null;
    isEmpty: boolean;
    loadingText: string;
    emptyText: string;
    children?: Snippet;
  }

  let {
    isLoading,
    error,
    isEmpty,
    loadingText,
    emptyText,
    children
  }: Props = $props();
</script>

{#if isLoading}
  <p class="ui-muted">{loadingText}</p>
{:else if error}
  <p class="ui-error" role="alert">{error}</p>
{:else if isEmpty}
  <p class="ui-muted">{emptyText}</p>
{:else if children}
  {@render children()}
{/if}
