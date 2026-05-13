<script lang="ts">
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import Ellipsis from 'lucide-svelte/icons/ellipsis';
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

  let menuOpen = $state(false);

  function getToggleTooltip(): string {
    return $relayMode === 'twitch' ? 'Switch to YouTube Relay' : 'Switch to Twitch Relay';
  }

  function toggleMenu(): void {
    menuOpen = !menuOpen;
  }

  function closeMenu(): void {
    menuOpen = false;
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
            <span class="status-dot connected" aria-hidden="true"></span>
            Linked as <strong>{twitchStatus.display_name || twitchStatus.login}</strong>
          {:else}
            <span class="status-dot disconnected" aria-hidden="true"></span>
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
      <!-- Desktop: inline buttons -->
      <div class="header-actions-inline">
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

      <!-- Mid-size: collapsed menu button -->
      <div class="header-actions-menu">
        <button
          type="button"
          class="ui-nav-chip menu-toggle"
          onclick={toggleMenu}
          aria-label="Menu"
          aria-expanded={menuOpen}
        >
          <Ellipsis size={18} />
        </button>

        {#if menuOpen}
          <div class="menu-dropdown" role="menu">
            {#if !isTwitchStatusLoaded}
              <button type="button" class="menu-item" disabled aria-busy="true">Loading...</button>
            {:else if twitchStatus.connected}
              <button type="button" class="menu-item" onclick={() => { closeMenu(); onDisconnectTwitch(); }} disabled={isTwitchBusy}>
                {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
              </button>
            {:else}
              <button type="button" class="menu-item" onclick={() => { closeMenu(); onConnectTwitch(); }}>
                Connect Twitch
              </button>
            {/if}
            <button type="button" class="menu-item" onclick={() => { closeMenu(); onSignOut(); }} disabled={isBusy}>
              Sign out
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</header>

{#if menuOpen}
  <div class="menu-backdrop" onclick={closeMenu} aria-hidden="true"></div>
{/if}

<style>
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
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

  .status-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    margin-right: 0.35rem;
    vertical-align: middle;
  }

  .status-dot.connected {
    background: var(--success);
  }

  .status-dot.disconnected {
    background: color-mix(in srgb, var(--muted) 60%, var(--danger));
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
    position: relative;
  }

  .header-actions-inline {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-actions-menu {
    display: none;
    position: relative;
  }

  .menu-dropdown {
    position: absolute;
    top: calc(100% + 0.5rem);
    right: 0;
    min-width: 10rem;
    background: var(--surface);
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.6rem;
    padding: 0.35rem;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    z-index: 50;
  }

  .menu-item {
    width: 100%;
    text-align: left;
    padding: 0.55rem 0.75rem;
    border: 0;
    border-radius: 0.4rem;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .menu-item:hover:not(:disabled) {
    background: var(--accent-soft);
  }

  .menu-item:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
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
    cursor: pointer;
  }

  .header-actions :global(.ui-nav-chip:hover:not(:disabled)) {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  .header-actions :global(.ui-nav-chip:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .menu-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.4rem;
    min-width: 2.35rem;
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

  /* Collapse to menu on mid-size screens */
  @media (max-width: 768px) and (min-width: 601px) {
    .header-actions-inline {
      display: none;
    }

    .header-actions-menu {
      display: block;
    }
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

    .header-actions-inline {
      display: flex;
      flex-wrap: wrap;
    }

    .header-actions-menu {
      display: none;
    }
  }
</style>
