<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import QRCode from 'qrcode';
  import { relayMode } from '$lib/stores';

  import AppHeader from '$lib/components/home/AppHeader.svelte';
  import AuthPanel from '$lib/components/home/AuthPanel.svelte';
  import YouTubeModeView from '$lib/components/home/YouTubeModeView.svelte';
  import TwitchChannelsView from '$lib/components/home/TwitchChannelsView.svelte';
  import RecordingsOverview from '$lib/components/home/RecordingsOverview.svelte';
  import ConfirmRemoveDialog from '$lib/components/home/ConfirmRemoveDialog.svelte';

  import type {
    ChannelEntry,
    ChannelStatus,
    RecordingRule,
    ActiveRecording,
    RecordingFileEntry,
    TwitchStatusResponse
  } from '$lib/api-client/types';

  import {
    addChannel,
    claimQrSession,
    createQrSession,
    createWatchTicket,
    deleteRecordingFile,
    disconnectTwitch,
    getChannels,
    getLiveStatus,
    getQrStatus,
    getRecordingRules,
    getRecordings,
    getSessionState,
    getVersion,
    getTwitchConnectUrl,
    getTwitchStatus,
    login,
    logout,
    pinRecordingFile,
    removeChannel,
    startRecording,
    stopRecording,
    unpinRecordingFile,
    upsertRecordingRule
  } from '$lib/api';

  type AuthMode = 'checking' | 'authenticated' | 'unauthenticated';
  type YouTubeViewMode = 'subscriptions' | 'playlists';
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
  let recordingRules = $state<Record<string, RecordingRule>>({});
  let activeRecordings = $state<Record<string, ActiveRecording>>({});
  let completedRecordings = $state<Array<RecordingFileEntry>>([]);
  let incompleteRecordings = $state<Array<RecordingFileEntry>>([]);
  let currentView = $state<'channels' | 'recordings'>('channels');
  let recordingsChannelFilter = $state('all');
  let deletingRecordingKey = $state<string | null>(null);
  let pinningRecordingKey = $state<string | null>(null);

  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let isAddingChannel = $state(false);

  let confirmRemoveChannel = $state<string | null>(null);
  let isRemovingChannel = $state(false);

  // YouTube view state
  let youtubeViewMode = $state<YouTubeViewMode>('subscriptions');

  // QR Login state
  let loginMode = $state<'code' | 'qr'>('code');
  let qrToken = $state<string | null>(null);
  let qrDataUrl = $state<string | null>(null);
  let qrPollInterval: ReturnType<typeof setInterval> | null = null;

  let pollInterval: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    relayMode.init();
    liveOnly = loadLiveOnlyPreference();

    // Parse URL parameters for view state
    const params = new URLSearchParams(window.location.search);
    const twitchView = params.get('twitch');
    const youtubeView = params.get('youtube');

    if (twitchView === 'recordings') {
      currentView = 'recordings';
      relayMode.setTwitch();
    } else if (twitchView === 'channels') {
      currentView = 'channels';
      relayMode.setTwitch();
    } else if (youtubeView === 'subscriptions') {
      youtubeViewMode = 'subscriptions';
      relayMode.setYoutube();
    } else if (youtubeView === 'playlists') {
      youtubeViewMode = 'playlists';
      relayMode.setYoutube();
    } else {
      // Default to twitch channels
      currentView = 'channels';
      relayMode.setTwitch();
    }

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
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
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

  function onLiveOnlyChange(_value: boolean): void {
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

  async function switchToQrMode(): Promise<void> {
    loginMode = 'qr';
    errorMessage = null;
    await generateQrCode();
  }

  function switchToCodeMode(): void {
    loginMode = 'code';
    errorMessage = null;
    // Stop polling when leaving QR mode
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
      qrPollInterval = null;
    }
    qrToken = null;
    qrDataUrl = null;
  }

  async function generateQrCode(): Promise<void> {
    try {
      const session = await createQrSession();
      qrToken = session.token;

      // Generate QR code data URL
      const qrUrl = `${window.location.origin}/qr-login/${encodeURIComponent(session.token)}`;
      qrDataUrl = await QRCode.toDataURL(qrUrl, {
        width: 200,
        margin: 2,
        color: {
          dark: '#c8d3f5',
          light: '#2f334d'
        }
      });

      // Start polling for status
      startQrPolling();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to generate QR code');
      loginMode = 'code';
    }
  }

  function startQrPolling(): void {
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
    }

    if (!qrToken) return;

    qrPollInterval = setInterval(async () => {
      if (!qrToken) return;

      try {
        const status = await getQrStatus(qrToken);
        if (status.status === 'authenticated') {
          // Stop polling and claim the session to get the cookie set
          if (qrPollInterval) {
            clearInterval(qrPollInterval);
            qrPollInterval = null;
          }
          try {
            await claimQrSession(qrToken);
            // Now we have the session cookie, reload to show authenticated UI
            window.location.reload();
          } catch (err) {
            errorMessage = readMessage(err, 'failed to claim session');
            // Go back to code mode on failure
            switchToCodeMode();
          }
        }
      } catch {
        // Ignore polling errors, session might just not be ready yet
      }
    }, 3000); // Poll every 3 seconds
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

  function openRecordingPlayer(file: RecordingFileEntry): void {
    const query = new URLSearchParams({
      channel_login: file.channel_login,
      filename: file.filename
    });
    window.location.assign(`/recordings/play?${query.toString()}`);
  }

  async function removeRecordingFile(bucket: 'completed' | 'incomplete', file: RecordingFileEntry): Promise<void> {
    const shouldDelete = window.confirm(`Delete ${file.filename}?`);
    if (!shouldDelete) {
      return;
    }

    const key = `${bucket}:${file.channel_login}:${file.filename}`;
    deletingRecordingKey = key;
    errorMessage = null;
    try {
      await deleteRecordingFile({
        bucket,
        channel_login: file.channel_login,
        filename: file.filename
      });
      await loadRecordingState();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to delete recording');
    } finally {
      deletingRecordingKey = null;
    }
  }

  async function toggleRecordingPin(file: RecordingFileEntry): Promise<void> {
    const key = `completed:${file.channel_login}:${file.filename}`;
    pinningRecordingKey = key;
    errorMessage = null;

    try {
      if (file.pinned) {
        await unpinRecordingFile({
          bucket: 'completed',
          channel_login: file.channel_login,
          filename: file.filename
        });
      } else {
        await pinRecordingFile({
          bucket: 'completed',
          channel_login: file.channel_login,
          filename: file.filename
        });
      }
      await loadRecordingState();
    } catch (err) {
      errorMessage = readMessage(err, file.pinned ? 'failed to unpin recording' : 'failed to pin recording');
    } finally {
      pinningRecordingKey = null;
    }
  }

  function selectedQuality(channelLogin: string): string {
    return recordingRules[channelLogin]?.quality || 'best';
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
        max_duration_minutes: current?.max_duration_minutes ?? null,
        keep_last_videos: current?.keep_last_videos ?? null
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

  function openChannelSetup(channelLogin: string): void {
    window.location.assign(`/channels/${channelLogin}`);
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

  function handleToggleMode(): void {
    relayMode.toggle();
  }
</script>

<svelte:head>
  <title>Twitch Relay</title>
</svelte:head>

<main class={`shell ${$relayMode === 'youtube' ? 'theme-youtube' : 'theme-twitch'}`}>
  <section class="panel">
    <AppHeader
      {authMode}
      relayMode={$relayMode}
      {twitchStatus}
      {isTwitchBusy}
      {isBusy}
      onToggleMode={handleToggleMode}
      onConnectTwitch={connectTwitch}
      onDisconnectTwitch={unlinkTwitch}
      onSignOut={signOut}
    />

    {#if errorMessage}
      <p class="ui-error" role="alert">{errorMessage}</p>
    {/if}

    {#if authMode === 'checking'}
      <p class="ui-muted">Checking session...</p>
    {:else if authMode === 'unauthenticated'}
      <AuthPanel
        {loginMode}
        {accessCode}
        {qrDataUrl}
        {isBusy}
        onSubmitLogin={submitLogin}
        onSwitchToQr={switchToQrMode}
        onSwitchToCode={switchToCodeMode}
        onUpdateAccessCode={(value) => (accessCode = value)}
      />
    {:else}
      {#if $relayMode === 'youtube'}
        <YouTubeModeView
          {youtubeViewMode}
          onViewModeChange={(mode) => (youtubeViewMode = mode)}
        />
      {:else}
        {#if currentView === 'channels'}
          <TwitchChannelsView
            {channels}
            {liveStatus}
            bind:liveOnly
            {showAddForm}
            {newChannelLogin}
            {isAddingChannel}
            {watchingChannel}
            {recordingRules}
            {activeRecordings}
            {liveStatusError}
            onLiveOnlyChange={onLiveOnlyChange}
            onOpenRecordings={openRecordingsOverview}
            onShowAddForm={() => (showAddForm = true)}
            onCancelAddForm={cancelAddChannel}
            onSubmitAddChannel={submitAddChannel}
            onUpdateNewChannelLogin={(value) => (newChannelLogin = value)}
            onOpenChannelSetup={openChannelSetup}
            onStartWatching={startWatching}
            onToggleAutoRecord={toggleAutoRecord}
            onToggleManualRecording={toggleManualRecording}
            onPromptRemoveChannel={promptRemoveChannel}
          />
        {:else}
          <RecordingsOverview
            {activeRecordings}
            {completedRecordings}
            {incompleteRecordings}
            {recordingsChannelFilter}
            {deletingRecordingKey}
            {pinningRecordingKey}
            onBackToChannels={backToChannels}
            onUpdateFilter={(value) => (recordingsChannelFilter = value)}
            onOpenRecordingPlayer={openRecordingPlayer}
            onRemoveRecordingFile={removeRecordingFile}
            onToggleRecordingPin={toggleRecordingPin}
          />
        {/if}
      {/if}
    {/if}
  </section>
  <p class="app-version" aria-label="App version">v{appVersion}</p>
</main>

<ConfirmRemoveDialog
  channelLogin={confirmRemoveChannel}
  isRemoving={isRemovingChannel}
  onConfirm={confirmRemove}
  onCancel={cancelRemove}
/>

<style>
  .shell {
    --bg: #1e2030;
    --bg-soft: #222436;
    --surface: #2f334d;
    --surface-2: #3b4261;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --accent: #82aaff;
    --accent-hover: #a8c5ff;
    --accent-2: #c099ff;
    --accent-soft: rgba(130, 170, 255, 0.16);
    --accent-border: rgba(130, 170, 255, 0.38);
    --focus-ring: rgba(130, 170, 255, 0.3);
    --success: #c3e88d;
    --warn: #ffc777;
    --danger: #ff757f;
    --border: #444a73;
    --ring: rgba(130, 170, 255, 0.45);
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(
      circle at 20% -10%,
      color-mix(in srgb, var(--surface-2) 88%, black) 0%,
      var(--bg-soft) 45%,
      var(--bg) 100%
    );
    color: var(--fg);
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell.theme-youtube {
    --bg: #2a171d;
    --bg-soft: #342029;
    --surface: #462a35;
    --surface-2: #5a3342;
    --border: #7b3f52;
    --accent: #ff0033;
    --accent-hover: #cc0029;
    --accent-soft: rgba(255, 0, 51, 0.16);
    --accent-border: rgba(255, 0, 51, 0.35);
    --focus-ring: rgba(255, 0, 51, 0.5);
    --ring: rgba(255, 0, 51, 0.35);
  }

  .shell {
    position: relative;
    height: 100dvh;
    box-sizing: border-box;
    display: grid;
    justify-items: center;
    align-content: start;
    padding: 1rem 1rem 3rem;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;

    &::-webkit-scrollbar {
      display: none;
    }
  }

  .app-version {
    position: absolute;
    left: 50%;
    bottom: 0.75rem;
    transform: translateX(-50%);
    margin: 0;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    color: color-mix(in srgb, var(--muted) 78%, var(--fg));
    pointer-events: none;
    user-select: none;
  }

  .app-version {
    position: absolute;
    left: 50%;
    bottom: 0.75rem;
    transform: translateX(-50%);
    margin: 0;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    color: color-mix(in srgb, var(--muted) 78%, var(--fg));
    pointer-events: none;
    user-select: none;
  }

  .panel {
    width: min(46rem, 100%);
    background: color-mix(in srgb, var(--surface) 82%, var(--bg-soft));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  /* .error, .muted styles now provided by app.css via .ui-error and .ui-muted */

  @media (max-width: 600px) {
    .panel {
      padding: 1rem;
    }
  }
</style>
