<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  import {
    addChannel,
    createWatchTicket,
    disconnectTwitch,
    getChannels,
    getLiveStatus,
    getSessionState,
    getVersion,
    getTwitchConnectUrl,
    getTwitchStatus,
    login,
    logout,
    removeChannel,
    type ChannelEntry,
    type ChannelStatus,
    type TwitchStatusResponse
  } from '$lib/api';

  type AuthMode = 'checking' | 'authenticated' | 'unauthenticated';
  const LIVE_ONLY_PREF_KEY = 'twitchRelay.liveOnly';

  let authMode = $state<AuthMode>('checking');
  let isBusy = $state(false);
  let errorMessage = $state<string | null>(null);
  let accessCode = $state('');
  let channels = $state<Array<ChannelEntry>>([]);
  let watchingChannel = $state<string | null>(null);
  let liveStatus = $state<Record<string, ChannelStatus>>({});
  let liveStatusError = $state<string | null>(null);
  let liveOnly = $state(false);
  let twitchStatus = $state<TwitchStatusResponse>({ connected: false, scopes: [] });
  let isTwitchBusy = $state(false);
  let appVersion = $state('?');

  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let isAddingChannel = $state(false);

  let confirmRemoveChannel = $state<string | null>(null);
  let isRemovingChannel = $state(false);

  let pollInterval: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    liveOnly = loadLiveOnlyPreference();
    void loadVersion();
    await initialize();
  });

  async function loadVersion(): Promise<void> {
    try {
      const payload = await getVersion();
      appVersion = payload.version;
    } catch {
      appVersion = '?';
    }
  }

  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
  });

  async function initialize(): Promise<void> {
    errorMessage = null;
    authMode = 'checking';

    try {
      const authenticated = await getSessionState();
      authMode = authenticated ? 'authenticated' : 'unauthenticated';
      if (authenticated) {
        await loadTwitchStatus();
        await loadChannels();
        startPolling();
      }
    } catch (err) {
      authMode = 'unauthenticated';
      errorMessage = readMessage(err, 'failed to initialize session');
    }
  }

  function startPolling(): void {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
    pollInterval = setInterval(async () => {
      await loadLiveStatus();
    }, 60000);
  }

  async function loadLiveStatus(): Promise<void> {
    try {
      const status = await getLiveStatus();
      liveStatus = status.channels;
      liveStatusError = null;
    } catch {
      liveStatusError = 'Live status refresh is temporarily unavailable';
    }
  }

  function visibleChannels(): Array<ChannelEntry> {
    if (!liveOnly) {
      return channels;
    }

    return channels.filter((channel) => Boolean(liveStatus[channel.login]?.live));
  }

  function loadLiveOnlyPreference(): boolean {
    try {
      return window.localStorage.getItem(LIVE_ONLY_PREF_KEY) === '1';
    } catch {
      return false;
    }
  }

  function saveLiveOnlyPreference(value: boolean): void {
    try {
      window.localStorage.setItem(LIVE_ONLY_PREF_KEY, value ? '1' : '0');
    } catch {
      // Ignore storage failures and keep in-memory state
    }
  }

  function onLiveOnlyChange(): void {
    saveLiveOnlyPreference(liveOnly);
  }

  async function submitLogin(event: SubmitEvent): Promise<void> {
    event.preventDefault();

    const normalized = accessCode.trim();
    if (!normalized) {
      errorMessage = 'access code is required';
      return;
    }

    isBusy = true;
    errorMessage = null;

    try {
      await login(normalized);
      accessCode = '';
      authMode = 'authenticated';
      await loadTwitchStatus();
      await loadChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'login failed');
    } finally {
      isBusy = false;
    }
  }

  async function loadChannels(): Promise<void> {
    try {
      channels = await getChannels();
      await loadLiveStatus();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to load channels');
      channels = [];
    }
  }

  async function loadTwitchStatus(): Promise<void> {
    try {
      twitchStatus = await getTwitchStatus();
    } catch (err) {
      twitchStatus = { connected: false, scopes: [] };
      errorMessage = readMessage(err, 'failed to load Twitch status');
    }
  }

  function connectTwitch(): void {
    window.location.assign(getTwitchConnectUrl());
  }

  async function unlinkTwitch(): Promise<void> {
    isTwitchBusy = true;
    errorMessage = null;
    try {
      await disconnectTwitch();
      twitchStatus = { connected: false, scopes: [] };
      await loadChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to disconnect Twitch account');
    } finally {
      isTwitchBusy = false;
    }
  }

  async function startWatching(channelLogin: string): Promise<void> {
    watchingChannel = channelLogin;
    errorMessage = null;

    try {
      const ticket = await createWatchTicket(channelLogin);
      window.location.assign(ticket.watch_url);
    } catch (err) {
      errorMessage = readMessage(err, `failed to open ${channelLogin}`);
    } finally {
      watchingChannel = null;
    }
  }

  async function submitAddChannel(event: SubmitEvent): Promise<void> {
    event.preventDefault();

    const normalized = newChannelLogin.trim().toLowerCase();
    if (!normalized) {
      errorMessage = 'channel name is required';
      return;
    }

    isAddingChannel = true;
    errorMessage = null;

    try {
      await addChannel(normalized);
      newChannelLogin = '';
      showAddForm = false;
      await loadChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to add channel');
    } finally {
      isAddingChannel = false;
    }
  }

  function cancelAddChannel(): void {
    showAddForm = false;
    newChannelLogin = '';
    errorMessage = null;
  }

  function promptRemoveChannel(login: string): void {
    confirmRemoveChannel = login;
  }

  async function confirmRemove(): Promise<void> {
    if (!confirmRemoveChannel) return;

    isRemovingChannel = true;
    errorMessage = null;

    try {
      await removeChannel(confirmRemoveChannel);
      confirmRemoveChannel = null;
      await loadChannels();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to remove channel');
    } finally {
      isRemovingChannel = false;
    }
  }

  function cancelRemove(): void {
    confirmRemoveChannel = null;
  }

  async function signOut(): Promise<void> {
    isBusy = true;
    errorMessage = null;

    try {
      await logout();
      authMode = 'unauthenticated';
      channels = [];
      twitchStatus = { connected: false, scopes: [] };
    } catch (err) {
      errorMessage = readMessage(err, 'logout failed');
    } finally {
      isBusy = false;
    }
  }

  function readMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }

    return fallback;
  }
</script>

<svelte:head>
  <title>Twitch Relay</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="panel-header">
      <div class="panel-title">
        <p class="eyebrow">Private Deck</p>
        <h1>Twitch Relay</h1>
        {#if authMode === 'authenticated'}
          <p class="header-subtle">
            {#if twitchStatus.connected}
              Linked as <strong>{twitchStatus.display_name || twitchStatus.login}</strong>
            {:else}
              Twitch not connected
            {/if}
          </p>
        {/if}
      </div>

      {#if authMode === 'authenticated'}
        <div class="header-actions">
          {#if twitchStatus.connected}
            <button type="button" class="ghost compact" onclick={unlinkTwitch} disabled={isTwitchBusy}>
              {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
            </button>
          {:else}
            <button type="button" class="compact" onclick={connectTwitch}>Connect Twitch</button>
          {/if}
          <button class="ghost compact" onclick={signOut} disabled={isBusy}>
            Sign out
          </button>
        </div>
      {/if}
    </header>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}

    {#if authMode === 'checking'}
      <p class="muted">Checking session...</p>
    {:else if authMode === 'unauthenticated'}
      <form class="login-form" onsubmit={submitLogin}>
        <label for="access-code">Access code</label>
        <input
          id="access-code"
          type="password"
          bind:value={accessCode}
          placeholder="Enter shared access code"
          autocomplete="current-password"
        />
        <button type="submit" disabled={isBusy}>{isBusy ? 'Signing in...' : 'Sign in'}</button>
      </form>
    {:else}
      <div class="channels-header">
        <div class="channels-title-row">
          <span class="channels-label">Channels</span>
          <label class="live-only-switch" aria-label="Show only live channels">
            <span class="switch-text">Live only</span>
            <input class="switch-input" type="checkbox" bind:checked={liveOnly} onchange={onLiveOnlyChange} />
            <span class="switch-track" aria-hidden="true">
              <span class="switch-knob"></span>
            </span>
          </label>
        </div>
        {#if !showAddForm}
          <button type="button" class="add-btn" onclick={() => showAddForm = true}>
            + Add channel
          </button>
        {/if}
      </div>

      {#if liveStatusError}
        <p class="live-status-warning">{liveStatusError}</p>
      {/if}

      {#if showAddForm}
        <form class="add-form" onsubmit={submitAddChannel}>
          <input
            type="text"
            bind:value={newChannelLogin}
            placeholder="channel_login"
            autocomplete="off"
            spellcheck="false"
          />
          <button type="submit" disabled={isAddingChannel}>
            {isAddingChannel ? 'Adding...' : 'Add'}
          </button>
          <button type="button" class="ghost" onclick={cancelAddChannel}>
            Cancel
          </button>
        </form>
      {/if}

      <div class="channels">
        {#if visibleChannels().length === 0}
          <p class="muted">{liveOnly ? 'No channels are live right now.' : 'No channels configured yet.'}</p>
        {:else}
          {#each visibleChannels() as channel (channel.login)}
            {@const status = liveStatus[channel.login]}
            <article class="channel-card">
              {#if channel.image_url}
                <img class="channel-avatar" src={channel.image_url} alt={channel.login} />
              {/if}
              <div class="channel-info">
                <div class="channel-name-row">
                  <p class="channel-name">{status?.display_name || channel.display_name || channel.login}</p>
                  {#if status?.live}
                    <span class="live-badge">
                      <span class="live-dot"></span>
                      LIVE
                    </span>
                  {/if}
                </div>
                <p class="channel-meta">{channel.source === 'manual' ? 'Manual' : channel.source === 'followed' ? 'Followed' : 'Manual + Followed'}</p>
                {#if status?.live && status.title}
                  <p class="channel-title" title={status.title}>{status.title}</p>
                {/if}
                <p class="channel-subtitle">
                  {#if status?.live && status.game}
                    🎮 {status.game}
                  {:else if status?.live && status.viewer_count}
                    👁 {status.viewer_count.toLocaleString()} viewers
                  {:else}
                    Allowlisted channel
                  {/if}
                </p>
              </div>
              <div class="channel-actions">
                {#if channel.removable}
                  <button
                    type="button"
                    class="remove-btn"
                    onclick={() => promptRemoveChannel(channel.login)}
                    title="Remove channel"
                  >
                    &times;
                  </button>
                {/if}
                <button
                  type="button"
                  onclick={() => startWatching(channel.login)}
                  disabled={watchingChannel === channel.login}
                >
                  {watchingChannel === channel.login ? 'Opening...' : 'Watch'}
                </button>
              </div>
            </article>
          {/each}
        {/if}
      </div>
    {/if}
  </section>
</main>

<p class="app-version" aria-label="App version">v{appVersion}</p>

{#if confirmRemoveChannel}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-overlay" onclick={cancelRemove} role="presentation">
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <p class="modal-text">Remove <strong>{confirmRemoveChannel}</strong> from the channel list?</p>
      <div class="modal-actions">
        <button type="button" class="ghost" onclick={cancelRemove} disabled={isRemovingChannel}>
          Cancel
        </button>
        <button type="button" class="danger" onclick={confirmRemove} disabled={isRemovingChannel}>
          {isRemovingChannel ? 'Removing...' : 'Remove'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Tokyo Night Moon theme tokens */
  :global(body) {
    --bg: #1e2030;
    --bg-soft: #222436;
    --surface: #2f334d;
    --surface-2: #3b4261;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --accent: #82aaff;
    --accent-2: #c099ff;
    --success: #c3e88d;
    --warn: #ffc777;
    --danger: #ff757f;
    --border: #444a73;
    --ring: rgba(130, 170, 255, 0.45);
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(circle at 20% -10%, #3b4261 0%, #222436 45%, #1e2030 100%);
    color: var(--fg);
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1rem 3rem;
  }

  .app-version {
    position: fixed;
    left: 50%;
    bottom: 0.75rem;
    transform: translateX(-50%);
    margin: 0;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    color: rgba(190, 206, 234, 0.72);
    pointer-events: none;
    user-select: none;
  }

  .panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, rgba(47, 51, 77, 0.95), rgba(34, 36, 54, 0.95));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

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

  .compact {
    padding: 0.52rem 0.8rem;
    font-size: 0.9rem;
  }

  h1 {
    margin: 0.2rem 0 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .error {
    margin: 0 0 1rem;
    padding: 0.7rem 0.8rem;
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    border-radius: 0.6rem;
    color: color-mix(in srgb, var(--danger) 72%, white);
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .login-form {
    display: grid;
    gap: 0.75rem;
  }

  .login-form label {
    font-weight: 600;
    color: var(--fg);
  }

  input {
    border: 1px solid rgba(160, 181, 216, 0.35);
    background: rgba(8, 12, 19, 0.9);
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.7rem 0.8rem;
    font: inherit;
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

  .ghost {
    background: transparent;
    border: 1px solid rgba(162, 182, 217, 0.35);
    color: var(--fg);
  }

  .channels-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .channels-title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .channels-label {
    font-weight: 600;
    color: var(--fg);
  }

  .live-only-switch {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--muted);
    font-size: 0.82rem;
    cursor: pointer;
    user-select: none;
    line-height: 1;
  }

  .switch-text {
    color: var(--muted);
    letter-spacing: 0.01em;
  }

  .switch-input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  .switch-track {
    width: 2.6rem;
    height: 1.45rem;
    border-radius: 999px;
    background: rgba(149, 170, 206, 0.3);
    border: 1px solid rgba(162, 182, 217, 0.4);
    display: inline-flex;
    align-items: center;
    padding: 0.11rem;
    transition: background-color 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
  }

  .switch-knob {
    width: 1.12rem;
    height: 1.12rem;
    border-radius: 50%;
    background: var(--fg);
    box-shadow: 0 1px 5px rgba(0, 0, 0, 0.28);
    transform: translateX(0);
    transition: transform 0.18s ease;
  }

  .switch-input:checked + .switch-track {
    background: color-mix(in srgb, var(--accent) 80%, var(--accent-2));
    border-color: color-mix(in srgb, var(--accent) 68%, white);
  }

  .switch-input:checked + .switch-track .switch-knob {
    transform: translateX(1.12rem);
  }

  .switch-input:focus-visible + .switch-track {
    box-shadow: 0 0 0 3px rgba(255, 111, 97, 0.28);
  }

  .switch-input:disabled + .switch-track {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .live-only-switch input {
    margin: 0;
  }

  .live-status-warning {
    margin: 0 0 0.65rem;
    color: var(--warn);
    font-size: 0.8rem;
  }

  .add-btn {
    background: transparent;
    border: 1px dashed rgba(162, 182, 217, 0.4);
    color: var(--muted);
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
  }

  .add-btn:hover {
    border-color: rgba(162, 182, 217, 0.7);
    color: var(--fg);
  }

  .add-form {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .add-form input {
    flex: 1;
    text-transform: lowercase;
  }

  .channels {
    display: grid;
    gap: 0.75rem;
  }

  .channel-card {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.75rem;
    border: 1px solid rgba(156, 178, 215, 0.22);
    background: rgba(10, 16, 27, 0.78);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .channel-card > * {
    min-width: 0;
  }

  .channel-avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .channel-info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .channel-name {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    text-transform: lowercase;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    flex: 1;
  }

  .channel-name-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .channel-meta {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .live-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    background: color-mix(in srgb, var(--success) 86%, transparent);
    color: #1e2030;
    font-size: 0.65rem;
    line-height: 1;
    font-weight: 700;
    height: 1.2rem;
    padding: 0 0.45rem;
    border-radius: 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .live-dot {
    width: 6px;
    height: 6px;
    background: #1e2030;
    border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .channel-title {
    display: block;
    width: 100%;
    margin: 0.2rem 0 0;
    color: color-mix(in srgb, var(--fg) 85%, var(--muted));
    font-size: 0.82rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .channel-subtitle {
    margin: 0.15rem 0 0;
    color: var(--muted);
    font-size: 0.87rem;
  }

  .channel-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-self: end;
    flex-shrink: 0;
  }

  .remove-btn {
    background: transparent;
    border: none;
    color: var(--muted);
    font-size: 1.4rem;
    padding: 0.2rem 0.5rem;
    line-height: 1;
  }

  .remove-btn:hover {
    color: var(--danger);
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: linear-gradient(160deg, rgba(20, 28, 43, 0.98), rgba(13, 18, 28, 0.98));
    border: 1px solid rgba(164, 182, 216, 0.3);
    border-radius: 1rem;
    padding: 1.5rem;
    max-width: 20rem;
    width: 90%;
  }

  .modal-text {
    margin: 0 0 1.25rem;
    color: var(--fg);
    line-height: 1.5;
  }

  .modal-text strong {
    text-transform: lowercase;
    color: var(--danger);
  }

  .modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .danger {
    background: color-mix(in srgb, var(--danger) 92%, #1e2030);
  }

  @media (max-width: 600px) {
    .panel {
      padding: 1rem;
    }

    .panel-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .header-actions {
      width: 100%;
      justify-content: flex-start;
    }

    .channel-card {
      display: flex;
      align-items: flex-start;
      flex-direction: column;
    }

    .channels-title-row {
      flex-wrap: wrap;
    }

    .channel-actions {
      width: 100%;
    }

    .channel-actions button:not(.remove-btn) {
      flex: 1;
    }

    .add-form {
      flex-wrap: wrap;
    }

    .add-form input {
      width: 100%;
    }
  }
</style>
