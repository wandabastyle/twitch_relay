<script lang="ts">
  import { relayMode } from '$lib/stores';
  import type { AppHeaderProps } from './types';

  let {
    authMode,
    twitchStatus,
    isTwitchStatusLoaded,
    isTwitchBusy,
    isBusy,
    onToggleMode,
    onConnectTwitch,
    onDisconnectTwitch,
    onSignOut
  }: AppHeaderProps = $props();
</script>

<header class="panel-header">
  <div class="panel-title">
    {#if authMode === 'authenticated'}
      <div class="mode-toggle" role="group" aria-label="Select relay mode">
        <button
          type="button"
          class="mode-segment"
          class:active={$relayMode === 'twitch'}
          onclick={() => $relayMode !== 'twitch' && onToggleMode()}
          aria-pressed={$relayMode === 'twitch'}
        >
          Twitch
        </button>
        <button
          type="button"
          class="mode-segment"
          class:active={$relayMode === 'youtube'}
          onclick={() => $relayMode !== 'youtube' && onToggleMode()}
          aria-pressed={$relayMode === 'youtube'}
        >
          YouTube
        </button>
      </div>
      <p class="header-subtle">
        {#if $relayMode === 'twitch'}
          {#if twitchStatus.connected}
            Linked as <strong>{twitchStatus.display_name || twitchStatus.login}</strong>
          {:else}
            Twitch not connected
          {/if}
        {:else}
          Invidious subscriptions
        {/if}
      </p>
    {:else}
      <p class="eyebrow">Private Deck</p>
      <h1>Twitch Relay</h1>
    {/if}
  </div>

  {#if authMode === 'authenticated'}
    <div class="header-actions">
      {#if $relayMode === 'twitch'}
        {#if !isTwitchStatusLoaded}
          <button type="button" class="ui-nav-chip" disabled aria-busy="true">Loading...</button>
        {:else if twitchStatus.connected}
          <button type="button" class="ui-nav-chip" onclick={onDisconnectTwitch} disabled={isTwitchBusy}>
            {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
          </button>
        {:else}
          <button type="button" class="ui-nav-chip" onclick={onConnectTwitch}>Connect Twitch</button>
        {/if}
      {/if}
      <button class="ui-nav-chip" onclick={onSignOut} disabled={isBusy}>
        Sign out
      </button>
    </div>
  {/if}
</header>

<style>
  .panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .panel-title {
    min-width: 0;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .header-subtle strong {
    color: var(--fg);
    font-weight: 700;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  /* .nav-chip-btn styles now provided by app.css via .ui-nav-chip */
  /* Local override needed to override generic button selector */
  .header-actions :global(.ui-nav-chip) {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    min-height: 2rem;
  }

  .header-actions :global(.ui-nav-chip:hover) {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  .header-actions :global(.ui-nav-chip:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .mode-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
    padding: 0.2rem;
    background: color-mix(in srgb, var(--surface) 60%, transparent);
    border: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    border-radius: 0.6rem;
  }

  .mode-segment {
    appearance: none;
    background: transparent;
    border: 0;
    border-radius: 0.4rem;
    padding: 0.35rem 0.75rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--muted);
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      color 0.15s ease;
  }

  .mode-segment:hover {
    color: var(--fg);
  }

  .mode-segment.active {
    background: var(--accent);
    color: #1e2030;
    font-weight: 600;
  }

  .mode-segment:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--focus-ring);
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  @media (max-width: 600px) {
    .panel-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .header-actions {
      width: 100%;
      justify-content: flex-start;
    }

  }
</style>
