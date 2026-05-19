<script lang="ts">
  import type { ActiveRecording } from '$lib/api-client/types';

  interface Props {
    activeList: ActiveRecording[];
    shownActive: ActiveRecording[];
  }

  let { activeList, shownActive }: Props = $props();
</script>

<section class="recordings-section">
  <h2>Active ({activeList.length})</h2>
  {#if activeList.length === 0}
    <p class="ui-muted section-empty">No active recordings right now.</p>
  {:else}
    <ul class="recordings-list">
      {#each shownActive as recording (recording.channel_login)}
        <li>
          <span class="entry-main">{recording.channel_login}</span>
          <span class="entry-meta">{recording.mode} · {recording.quality}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .recordings-section {
    border: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 62%, #0a101b);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .recordings-section h2 {
    margin: 0 0 0.55rem;
    font-size: 0.95rem;
    font-weight: 700;
  }

  .section-empty {
    margin: 0.5rem 0;
    padding: 0.5rem 0;
  }

  .recordings-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.45rem;
  }

  .recordings-list li {
    display: grid;
    gap: 0.1rem;
  }

  .entry-main {
    font-size: 0.88rem;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .entry-meta {
    font-size: 0.8rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }
</style>
