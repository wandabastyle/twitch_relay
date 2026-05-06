<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';

  const videoId = $derived($page.params.video_id ?? '');
  const embedUrl = $derived(`/api/youtube/embed/${encodeURIComponent(videoId)}`);

  function goBack() {
    if (videoId) {
      window.history.back();
      return;
    }
    goto('/');
  }
</script>

<svelte:head>
  <title>{videoId ? `${videoId} - YouTube Relay` : 'YouTube Relay'}</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="player-header">
      <div>
        <button type="button" class="nav-chip-btn" onclick={goBack}>Back to videos</button>
        <h1>YouTube video</h1>
      </div>
    </header>

    {#if videoId}
      <iframe
        class="player"
        src={embedUrl}
        title="Invidious video player"
        allow="autoplay; fullscreen; encrypted-media; picture-in-picture"
        allowfullscreen
        loading="eager"
        referrerpolicy="strict-origin-when-cross-origin"
      ></iframe>
    {:else}
      <div class="player error-box">
        <p class="error" role="alert">No video ID provided.</p>
      </div>
    {/if}
  </section>
</main>

<style>
  .shell {
    min-height: 100dvh;
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-content: start;
    padding: 1rem 1rem 3rem;
  }

  .panel {
    width: min(74rem, 96vw);
    background: color-mix(in srgb, var(--surface) 82%, var(--bg-soft));
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
    display: grid;
    gap: 0.8rem;
  }

  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .nav-chip-btn {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.6rem;
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1;
    margin-bottom: 0.5rem;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2rem;
  }

  .nav-chip-btn:hover {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.2rem, 3vw, 1.8rem);
    line-height: 1.3;
  }

  .player {
    width: 100%;
    aspect-ratio: 16 / 9;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: #000;
  }

  .error-box {
    min-height: 16rem;
    display: grid;
    place-items: center;
    padding: 1rem;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }

  @media (min-width: 1100px) {
    .shell {
      padding: 0.75rem 1rem;
    }
  }
</style>
