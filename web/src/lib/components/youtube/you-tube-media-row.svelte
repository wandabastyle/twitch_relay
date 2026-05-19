<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    onClick: () => void;
    visual: Snippet;
    title: string;
    meta?: Snippet;
    extraClass?: string;
  }

  const { extraClass, meta, onClick, title, visual }: Props = $props();

  // Use $derived for reactive extraClass to capture updates
  const extraClassString = $derived(extraClass ?? '');
</script>

<button
  type="button"
  class={`ui-card ui-card-interactive ui-media-row ${extraClassString}`}
  onclick={onClick}
>
  <div class="ui-media-visual">
    {@render visual()}
  </div>
  <div class="ui-media-main">
    <span class="ui-media-title" title={title}>{title}</span>
    {#if meta}
      {@render meta()}
    {/if}
  </div>
</button>
