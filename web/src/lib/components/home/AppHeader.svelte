<script lang="ts">
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
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

  function getToggleTooltip(): string {
    return $relayMode === 'twitch' ? 'Switch to YouTube Relay' : 'Switch to Twitch Relay';
  }
</script>

<header class="panel-header">
  <div class="panel-title">
    <p class="eyebrow">Private Deck</p>
    {#if authMode === 'authenticated'}
      <button
        type="button"
        class="relay-title-button"
        onclick={onToggleMode}
        aria-label="Toggle between Twitch and YouTube mode"
        title={getToggleTooltip()}
      >
        {#if $relayMode === 'twitch'}
          <h1>Twitch Relay</h1>
        {:else}
          <h1>YouTube Relay</h1>
        {/if}
        <span class="toggle-icon" aria-hidden="true">
          <ArrowLeftRight size={14} />
        </span>
      </button>
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
      <h1>Twitch Relay</h1>
    {/if}
  </div>

  {#if authMode === 'authenticated'}
    <div class="header-actions">
      {#if !isTwitchStatusLoaded}
        <button type="button" class="ui-nav-chip" disabled aria-busy="true">Loading...</button>
      {:else if twitchStatus.connected}
        <button type="button" class="ui-nav-chip" onclick={onDisconnectTwitch} disabled={isTwitchBusy}>
          {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
        </button>
      {:else}
        <button type="button" class="ui-nav-chip" onclick={onConnectTwitch}>Connect Twitch</button>
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
    align-items: center;
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

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.62rem 0.95rem;
    background: var(--accent);
    color: #1e2030;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .relay-title-button,
  .relay-title-button:hover,
  .relay-title-button:focus,
  .relay-title-button:active {
    text-decoration: none;
  }

  .relay-title-button {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 0;
    margin: 0;
    font: inherit;
    font-weight: inherit;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition: opacity 0.15s ease, transform 0.15s ease;
    color: var(--muted);
  }

  .relay-title-button:hover .toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  .relay-title-button:hover {
    text-decoration: none;
  }

  .relay-title-button:hover {
    color: var(--accent);
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
