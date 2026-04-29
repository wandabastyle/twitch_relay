<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  import {
    addChannel,
    createWatchTicket,
    disconnectTwitch,
    getChannels,
    getLiveStatus,
    getRecordingRules,
    getRecordings,
    getSessionState,
    getVersion,
    getTwitchConnectUrl,
    getTwitchStatus,
    login,
    logout,
    removeChannel,
    startRecording,
    stopRecording,
    upsertRecordingRule,
    type ActiveRecording,
    type ChannelEntry,
    type ChannelStatus,
    type RecordingFileEntry,
    type RecordingRule,
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
  const QUALITY_OPTIONS = ['best', 'source', '1080p60', '1080p', '720p60', '720p', '480p', '360p', '160p'];
  let recordingRules = $state<Record<string, RecordingRule>>({});
  let activeRecordings = $state<Record<string, ActiveRecording>>({});
  let selectedQualityByChannel = $state<Record<string, string>>({});
  let completedRecordings = $state<Array<RecordingFileEntry>>([]);
  let incompleteRecordings = $state<Array<RecordingFileEntry>>([]);
  let currentView = $state<'channels' | 'recordings'>('channels');

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
      await loadRecordingState();
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
      await loadRecordingState();
      await loadRecordingRules();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to load channels');
      channels = [];
    }
  }

  async function loadRecordingRules(): Promise<void> {
    try {
      const rules = await getRecordingRules();
      const next: Record<string, RecordingRule> = {};
      for (const rule of rules) {
        next[rule.channel_login] = rule;
      }
      recordingRules = next;
    } catch {
      // ignore transient rule loading failures
    }
  }

  async function loadRecordingState(): Promise<void> {
    try {
      const recordings = await getRecordings();
      const next: Record<string, ActiveRecording> = {};
      for (const recording of recordings.active) {
        next[recording.channel_login] = recording;
      }
      activeRecordings = next;
      completedRecordings = recordings.completed;
      incompleteRecordings = recordings.incomplete;
    } catch {
      // ignore transient recording state failures
    }
  }

  function openRecordingsOverview(): void {
    currentView = 'recordings';
    showAddForm = false;
  }

  function backToChannels(): void {
    currentView = 'channels';
  }

  function latestThree<T>(entries: Array<T>): Array<T> {
    return entries.slice(0, 3);
  }

  function selectedQuality(channelLogin: string): string {
    return selectedQualityByChannel[channelLogin] || recordingRules[channelLogin]?.quality || 'best';
  }

  async function toggleAutoRecord(channelLogin: string): Promise<void> {
    const current = recordingRules[channelLogin];
    const enabled = !current?.enabled;
    try {
      await upsertRecordingRule({
        channel_login: channelLogin,
        enabled,
        quality: selectedQuality(channelLogin),
        stop_when_offline: current?.stop_when_offline ?? true,
        max_duration_minutes: current?.max_duration_minutes ?? null
      });
      await loadRecordingRules();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to toggle auto-record');
    }
  }

  async function toggleManualRecording(channelLogin: string): Promise<void> {
    const active = activeRecordings[channelLogin];
    try {
      if (active) {
        await stopRecording(channelLogin);
      } else {
        await startRecording(channelLogin, selectedQuality(channelLogin), liveStatus[channelLogin]?.title);
      }
      await loadRecordingState();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to toggle recording');
    }
  }

  function onQualityChange(channelLogin: string, quality: string): void {
    selectedQualityByChannel = {
      ...selectedQualityByChannel,
      [channelLogin]: quality
    };
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
      {#if currentView === 'channels'}
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
          <div class="channels-actions">
            <button type="button" class="overview-btn" onclick={openRecordingsOverview}>
              Recordings overview
            </button>
            {#if !showAddForm}
              <button type="button" class="add-btn" onclick={() => showAddForm = true}>
                + Add channel
              </button>
            {/if}
          </div>
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
              <div class="channel-avatar-wrap">
                {#if channel.image_url}
                  <img class="channel-avatar" src={channel.image_url} alt={channel.login} />
                {:else}
                  <div class="channel-avatar fallback" aria-hidden="true">{channel.login.slice(0, 1)}</div>
                {/if}
              </div>

              <div class="channel-main">
                <div class="channel-main-top">
                  <p class="channel-name">{status?.display_name || channel.display_name || channel.login}</p>
                </div>
                <p class="channel-meta">{channel.source === 'manual' ? 'Manual' : channel.source === 'followed' ? 'Followed' : 'Manual + Followed'}</p>
                <div class="channel-main-bottom">
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
              </div>

              <div class="channel-side">
                <div class="channel-side-top">
                  {#if status?.live}
                    <span class="live-badge">
                      <span class="live-dot"></span>
                      LIVE
                    </span>
                  {/if}
                  <button
                    type="button"
                    class="watch-btn"
                    onclick={() => startWatching(channel.login)}
                    disabled={watchingChannel === channel.login}
                  >
                    {watchingChannel === channel.login ? 'Opening...' : 'Watch'}
                  </button>
                </div>

                <div class="channel-actions">
                  <div class="recording-controls">
                  <button
                    type="button"
                    class={`icon-btn clock-btn ${recordingRules[channel.login]?.enabled ? 'enabled' : ''}`}
                    title={recordingRules[channel.login]?.enabled ? 'Disable auto-record' : 'Enable auto-record'}
                    onclick={() => toggleAutoRecord(channel.login)}
                  >
                    ⏰
                  </button>
                  <button
                    type="button"
                    class={`icon-btn record-btn ${activeRecordings[channel.login]?.mode === 'manual' ? 'active-manual' : activeRecordings[channel.login]?.mode === 'auto' ? 'active-auto' : ''}`}
                    title={
                      activeRecordings[channel.login]?.mode === 'manual'
                        ? 'Stop manual recording'
                        : activeRecordings[channel.login]?.mode === 'auto'
                          ? 'Stop auto recording'
                          : 'Start recording now'
                    }
                    onclick={() => toggleManualRecording(channel.login)}
                  >
                    ⬤
                  </button>
                  <select
                    class="quality-select"
                    value={selectedQuality(channel.login)}
                    onchange={(event) => onQualityChange(channel.login, (event.currentTarget as HTMLSelectElement).value)}
                    aria-label={`Recording quality for ${channel.login}`}
                  >
                    {#each QUALITY_OPTIONS as quality (quality)}
                      <option value={quality}>{quality}</option>
                    {/each}
                  </select>
                  </div>
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
                </div>
              </div>
              </article>
            {/each}
          {/if}
        </div>
      {:else}
        {@const activeList = Object.values(activeRecordings)}
        <div class="recordings-view">
          <div class="recordings-header">
            <div>
              <p class="channels-label">Recordings overview</p>
              <p class="recordings-subtle">Recent recording activity and files</p>
            </div>
            <button type="button" class="ghost" onclick={backToChannels}>Back to channels</button>
          </div>

          <div class="recordings-grid">
            <section class="recordings-section">
              <h2>Active ({activeList.length})</h2>
              {#if activeList.length === 0}
                <p class="muted">No active recordings right now.</p>
              {:else}
                <ul class="recordings-list">
                  {#each latestThree(activeList) as recording (recording.channel_login)}
                    <li>
                      <span class="entry-main">{recording.channel_login}</span>
                      <span class="entry-meta">{recording.mode} · {recording.quality}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </section>

            <section class="recordings-section">
              <h2>Completed ({completedRecordings.length})</h2>
              {#if completedRecordings.length === 0}
                <p class="muted">No completed files yet.</p>
              {:else}
                <ul class="recordings-list">
                  {#each latestThree(completedRecordings) as file (file.path_display)}
                    <li>
                      <span class="entry-main" title={file.filename}>{file.filename}</span>
                      <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </section>

            <section class="recordings-section">
              <h2>Incomplete ({incompleteRecordings.length})</h2>
              {#if incompleteRecordings.length === 0}
                <p class="muted">No incomplete files.</p>
              {:else}
                <ul class="recordings-list">
                  {#each latestThree(incompleteRecordings) as file (file.path_display)}
                    <li>
                      <span class="entry-main" title={file.filename}>{file.filename}</span>
                      <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </section>
          </div>
        </div>
      {/if}
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
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }

  .channels-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
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

  .overview-btn {
    background: transparent;
    border: 1px solid rgba(162, 182, 217, 0.45);
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
  }

  .add-btn:hover {
    border-color: rgba(162, 182, 217, 0.7);
    color: var(--fg);
  }

  .overview-btn:hover {
    border-color: rgba(190, 206, 234, 0.72);
    background: rgba(17, 26, 41, 0.72);
  }

  .recordings-view {
    display: grid;
    gap: 0.85rem;
  }

  .recordings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .recordings-subtle {
    margin: 0.3rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  .recordings-grid {
    display: grid;
    gap: 0.75rem;
  }

  .recordings-section {
    border: 1px solid rgba(156, 178, 215, 0.22);
    background: rgba(10, 16, 27, 0.78);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .recordings-section h2 {
    margin: 0 0 0.55rem;
    font-size: 0.95rem;
    font-weight: 700;
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
  }

  .entry-meta {
    font-size: 0.8rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    grid-template-columns: 74px minmax(0, 1fr) auto;
    align-items: stretch;
    gap: 0.75rem;
    border: 1px solid rgba(156, 178, 215, 0.22);
    background: rgba(10, 16, 27, 0.78);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .channel-card > * {
    min-width: 0;
  }

  .channel-avatar-wrap {
    height: 100%;
    min-height: 74px;
    display: flex;
    align-items: center;
  }

  .channel-avatar {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    object-fit: cover;
    display: block;
    background: rgba(160, 181, 216, 0.2);
  }

  .channel-avatar.fallback {
    display: grid;
    place-items: center;
    text-transform: uppercase;
    font-weight: 700;
    color: var(--fg);
  }

  .channel-main {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-width: 0;
    overflow: hidden;
    min-height: 74px;
  }

  .channel-main-top {
    display: flex;
    align-items: center;
    min-height: 1.6rem;
    min-width: 0;
    overflow: hidden;
  }

  .channel-main-bottom {
    min-height: 2.15rem;
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
    font-size: 0.74rem;
    line-height: 1;
    font-weight: 700;
    height: 2rem;
    padding: 0 0.72rem;
    border-radius: 0.55rem;
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

  .channel-side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: flex-end;
    min-height: 74px;
    gap: 0.35rem;
  }

  .channel-side-top {
    min-height: 2rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .watch-btn {
    height: 2rem;
    border-radius: 0.55rem;
    min-width: 4.7rem;
    padding: 0 0.8rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9rem;
    font-weight: 700;
    letter-spacing: 0.01em;
  }

  .recording-controls {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    --ctrl-h: 2.35rem;
    --ctrl-r: 0.62rem;
    --ctrl-border: rgba(160, 181, 216, 0.32);
    --ctrl-bg: rgba(14, 22, 36, 0.92);
    --ctrl-fg: var(--fg);
  }

  .icon-btn {
    width: var(--ctrl-h);
    height: var(--ctrl-h);
    border: 1px solid var(--ctrl-border);
    border-radius: var(--ctrl-r);
    font-size: 0.9rem;
    line-height: 1;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--ctrl-bg);
    color: var(--ctrl-fg);
  }

  .clock-btn.enabled {
    background: color-mix(in srgb, var(--accent) 46%, var(--ctrl-bg));
    border-color: color-mix(in srgb, var(--accent) 68%, white);
    color: #eaf2ff;
  }

  .record-btn {
    color: color-mix(in srgb, var(--muted) 82%, var(--fg));
    background: color-mix(in srgb, var(--ctrl-bg) 88%, #1b2436);
    border-color: color-mix(in srgb, var(--ctrl-border) 72%, transparent);
  }

  .record-btn.active-auto {
    background: color-mix(in srgb, #f3b35f 74%, #1e2030);
    border-color: color-mix(in srgb, #f3b35f 76%, #fff);
    color: #fff;
  }

  .record-btn.active-manual {
    background: color-mix(in srgb, var(--danger) 74%, #1e2030);
    border-color: color-mix(in srgb, var(--danger) 75%, #fff);
    color: #fff;
  }

  .quality-select {
    width: 6.4rem;
    height: var(--ctrl-h);
    border: 1px solid var(--ctrl-border);
    background: var(--ctrl-bg);
    color: var(--ctrl-fg);
    border-radius: var(--ctrl-r);
    padding: 0 0.6rem;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 500;
  }

  .icon-btn:hover,
  .quality-select:hover {
    border-color: rgba(190, 206, 234, 0.52);
    background: color-mix(in srgb, var(--ctrl-bg) 82%, #101b30);
  }

  .icon-btn:focus-visible,
  .quality-select:focus-visible,
  .watch-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px rgba(130, 170, 255, 0.24);
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
      grid-template-columns: 64px minmax(0, 1fr);
      align-items: stretch;
    }

    .channel-avatar-wrap {
      grid-row: span 2;
      min-height: 96px;
    }

    .channel-avatar {
      width: 64px;
      height: 64px;
    }

    .channel-main {
      min-height: 0;
    }

    .channel-side {
      grid-column: 2;
      align-items: stretch;
      min-height: 0;
    }

    .channel-side-top {
      justify-content: flex-start;
    }

    .live-badge,
    .watch-btn {
      height: 1.9rem;
    }

    .live-badge {
      padding: 0 0.62rem;
      font-size: 0.7rem;
    }

    .channels-title-row {
      flex-wrap: wrap;
    }

    .channels-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .channels-actions {
      width: 100%;
      justify-content: flex-start;
    }

    .recordings-header {
      align-items: flex-start;
    }

    .channel-actions {
      width: 100%;
      gap: 0.45rem;
    }

    .channel-actions button:not(.remove-btn) {
      flex: 1;
    }

    .recording-controls {
      --ctrl-h: 2.15rem;
      --ctrl-r: 0.56rem;
      gap: 0.4rem;
    }

    .quality-select {
      width: 5.8rem;
      font-size: 0.82rem;
    }

    .watch-btn {
      min-width: 4.4rem;
      font-size: 0.84rem;
      padding: 0 0.65rem;
    }

    .add-form {
      flex-wrap: wrap;
    }

    .add-form input {
      width: 100%;
    }
  }
</style>
