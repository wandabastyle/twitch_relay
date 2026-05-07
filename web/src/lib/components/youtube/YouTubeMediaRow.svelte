<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    onClick: () => void;
    visual: Snippet;
    title: string;
    meta?: Snippet;
    extraClass?: string;
  }

  let { onClick, visual, title, meta, extraClass }: Props = $props();
</script>

<button
  type="button"
  class={`ui-card ui-card-interactive ui-media-row ${extraClass ?? ''}`}
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
