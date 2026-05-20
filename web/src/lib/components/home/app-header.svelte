<script lang="ts">
  import type { AppHeaderProps } from './types';
  import Ellipsis from 'lucide-svelte/icons/ellipsis';
  import RelayHeader from '$lib/components/shared/relay-header.svelte';

  const {
    authMode,
    isBusy,
    isTwitchBusy,
    isTwitchStatusLoaded,
    onConnectTwitch,
    onDisconnectTwitch,
    onSignOut,
    onToggleMode,
    relayMode,
    twitchStatus,
  }: AppHeaderProps = $props();

  let menuOpen = $state(false);

  const getToggleTooltip = (): string => {
    if (relayMode === 'twitch') {
      return 'Switch to YouTube Relay';
    }
    return 'Switch to Twitch Relay';
  };

  const getTitle = (): string => {
    if (relayMode === 'twitch') {
      return 'Twitch Relay';
    }
    return 'YouTube Relay';
  };

  const toggleMenu = (): void => {
    menuOpen = !menuOpen;
  };

  const closeMenu = (): void => {
    menuOpen = false;
  };
</script>

{#if authMode === 'authenticated'}
  {#snippet headerSubtitle()}
    {#if relayMode === 'twitch'}
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
  {/snippet}
  <RelayHeader
    eyebrow="Private Deck"
    title={getTitle()}
    onToggle={onToggleMode}
    toggleLabel={getToggleTooltip()}
    subtitleSnippet={headerSubtitle}
  >
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
  </RelayHeader>

  {#if menuOpen}
    <div class="menu-backdrop" onclick={closeMenu} aria-hidden="true"></div>
  {/if}
{:else}
  <header class="app-header-simple">
    <div class="app-header-title">
      <p class="app-header-eyebrow">Private Deck</p>
      <h1>Twitch Relay</h1>
    </div>
  </header>
{/if}

<style>
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

  /* Simple header for unauthenticated state */
  .app-header-simple {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .app-header-title {
    min-width: 0;
  }

  .app-header-eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .app-header-simple h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
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
