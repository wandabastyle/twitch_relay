<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { relayMode } from '$lib/stores';

  import AppHeader from '$lib/components/home/AppHeader.svelte';
  import AuthPanel from '$lib/components/home/AuthPanel.svelte';
  import YouTubeModeView from '$lib/components/home/YouTubeModeView.svelte';
  import TwitchChannelsView from '$lib/components/home/TwitchChannelsView.svelte';
  import RecordingsOverview from '$lib/components/home/RecordingsOverview.svelte';
  import ConfirmRemoveDialog from '$lib/components/home/ConfirmRemoveDialog.svelte';
  import AppVersion from '$lib/components/AppVersion.svelte';

  import { createAuthController } from '$lib/home/authController.svelte';
  import { createQrController } from '$lib/home/qrController.svelte';
  import { createChannelsController } from '$lib/home/channelsController.svelte';
  import { createRecordingsController } from '$lib/home/recordingsController.svelte';

  import type { RecordingFileEntry } from '$lib/api-client/types';

  import { loadLiveOnlyPreference, saveLiveOnlyPreference } from '$lib/home/preferences';
  import { parseInitialHomeView } from '$lib/home/routeView';

  type YouTubeViewMode = 'subscriptions' | 'playlists';

  // Global error state (shared across controllers)
  let errorMessage = $state<string | null>(null);

  function setError(message: string | null): void {
    errorMessage = message;
  }

  // Simple UI state (kept in component)
  let currentView = $state<'channels' | 'recordings'>('channels');
  let youtubeViewMode = $state<YouTubeViewMode>('subscriptions');
  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let confirmRemoveChannel = $state<string | null>(null);
  let liveOnly = $state(false);

  // Polling interval reference
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  // Controllers
  const channelsController = createChannelsController({
    setError,
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
    },
  });

  const recordingsController = createRecordingsController({ setError });

  const authController = createAuthController({
    setError,
    onAuthenticated: async () => {
      await channelsController.loadChannels();
      startPolling();
    },
  });

  const qrController = createQrController({
    setError,
    onQrAuthenticated: () => {
      window.location.reload();
    },
  });

  onMount(async () => {
    relayMode.init();
    liveOnly = loadLiveOnlyPreference();

    // Parse URL parameters for view state
    const initialView = parseInitialHomeView(window.location.search);
    if (initialView.relayMode === 'twitch') {
      currentView = initialView.twitchView;
      relayMode.setTwitch();
    } else {
      youtubeViewMode = initialView.youtubeView;
      relayMode.setYoutube();
    }

    await authController.initialize();
  });

  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
    qrController.cleanup();
  });

  function startPolling(): void {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
    pollInterval = setInterval(async () => {
      await channelsController.loadLiveStatus();
      await recordingsController.loadRecordingState();
    }, 60000);
  }

  function onLiveOnlyChange(_value: boolean): void {
    saveLiveOnlyPreference(liveOnly);
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

  function openChannelSetup(channelLogin: string): void {
    window.location.assign(`/channels/${channelLogin}`);
  }

  function promptRemoveChannel(login: string): void {
    confirmRemoveChannel = login;
  }

  async function confirmRemove(): Promise<void> {
    if (!confirmRemoveChannel) return;
    await channelsController.confirmRemoveChannel(confirmRemoveChannel);
    confirmRemoveChannel = null;
  }

  function cancelRemove(): void {
    confirmRemoveChannel = null;
  }

  async function submitAddChannel(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    await channelsController.submitAddChannel(newChannelLogin);
    newChannelLogin = '';
    showAddForm = false;
  }

  function cancelAddChannel(): void {
    showAddForm = false;
    newChannelLogin = '';
    setError(null);
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
      authMode={authController.authMode}
      relayMode={$relayMode}
      twitchStatus={channelsController.twitchStatus}
      isTwitchBusy={channelsController.isTwitchBusy}
      isBusy={authController.isBusy}
      onToggleMode={handleToggleMode}
      onConnectTwitch={channelsController.connectTwitch}
      onDisconnectTwitch={channelsController.unlinkTwitch}
      onSignOut={authController.signOut}
    />

    {#if errorMessage}
      <p class="ui-error" role="alert">{errorMessage}</p>
    {/if}

    {#if authController.authMode === 'checking'}
      <p class="ui-muted">Checking session...</p>
    {:else if authController.authMode === 'unauthenticated'}
      <AuthPanel
        loginMode={qrController.loginMode}
        accessCode={authController.accessCode}
        qrDataUrl={qrController.qrDataUrl}
        isBusy={authController.isBusy}
        onSubmitLogin={authController.submitLogin}
        onSwitchToQr={qrController.switchToQrMode}
        onSwitchToCode={qrController.switchToCodeMode}
        onUpdateAccessCode={authController.setAccessCode}
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
            channels={channelsController.channels}
            liveStatus={channelsController.liveStatus}
            bind:liveOnly
            {showAddForm}
            {newChannelLogin}
            isAddingChannel={channelsController.isAddingChannel}
            watchingChannel={channelsController.watchingChannel}
            recordingRules={recordingsController.recordingRules}
            activeRecordings={recordingsController.activeRecordings}
            liveStatusError={channelsController.liveStatusError}
            onLiveOnlyChange={onLiveOnlyChange}
            onOpenRecordings={openRecordingsOverview}
            onShowAddForm={() => (showAddForm = true)}
            onCancelAddForm={cancelAddChannel}
            onSubmitAddChannel={submitAddChannel}
            onUpdateNewChannelLogin={(value) => (newChannelLogin = value)}
            onOpenChannelSetup={openChannelSetup}
            onStartWatching={channelsController.startWatching}
            onToggleAutoRecord={recordingsController.toggleAutoRecord}
            onToggleManualRecording={(login) =>
              recordingsController.toggleManualRecording(
                login,
                recordingsController.selectedQuality(login),
                channelsController.liveStatus[login]?.title
              )}
            onPromptRemoveChannel={promptRemoveChannel}
          />
        {:else}
          <RecordingsOverview
            activeRecordings={recordingsController.activeRecordings}
            completedRecordings={recordingsController.completedRecordings}
            incompleteRecordings={recordingsController.incompleteRecordings}
            recordingsChannelFilter="all"
            deletingRecordingKey={recordingsController.deletingRecordingKey}
            pinningRecordingKey={recordingsController.pinningRecordingKey}
            onBackToChannels={backToChannels}
            onUpdateFilter={() => {}}
            onOpenRecordingPlayer={openRecordingPlayer}
            onRemoveRecordingFile={recordingsController.removeRecordingFile}
            onToggleRecordingPin={recordingsController.toggleRecordingPin}
          />
        {/if}
      {/if}
    {/if}
  </section>
  <AppVersion />
</main>

<ConfirmRemoveDialog
  channelLogin={confirmRemoveChannel}
  isRemoving={channelsController.isRemovingChannel}
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
