<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  import AppHeader from '$lib/components/home/AppHeader.svelte';
  import AuthPanel from '$lib/components/home/AuthPanel.svelte';
  import TwitchChannelsView from '$lib/components/home/TwitchChannelsView.svelte';
  import RecordingsOverview from '$lib/components/home/RecordingsOverview.svelte';
  import ConfirmRemoveDialog from '$lib/components/home/ConfirmRemoveDialog.svelte';

  import { createAuthController } from '$lib/home/authController.svelte';
  import { createQrController } from '$lib/home/qrController.svelte';
  import { createChannelsController } from '$lib/home/channelsController.svelte';
  import { createRecordingsController } from '$lib/home/recordingsController.svelte';
  import LoadedFade from '$lib/components/LoadedFade.svelte';

  import type { RecordingFileEntry } from '$lib/api-client/types';

  import { loadLiveOnlyPreference, saveLiveOnlyPreference } from '$lib/home/preferences';

  // Global error state (shared across controllers)
  let errorMessage = $state<string | null>(null);

  function setError(message: string | null): void {
    errorMessage = message;
  }

  // Simple UI state (kept in component)
  let currentView = $state<'channels' | 'recordings'>('channels');
  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let confirmRemoveChannel = $state<string | null>(null);
  let liveOnly = $state(false);
  let recordingsChannelFilter = $state<string>('all');

  // Key to force re-mount of main view content for fade animation on view changes
  const viewKey = $derived(`twitch:${currentView}`);

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
    liveOnly = loadLiveOnlyPreference();
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
    window.location.assign(`/twitch/recordings/play?${query.toString()}`);
  }

  function openChannelSetup(channelLogin: string): void {
    window.location.assign(`/twitch/channels/${channelLogin}`);
  }

  function promptRemoveChannel(login: string): void {
    confirmRemoveChannel = login;
  }

  function onUpdateRecordingsFilter(value: string): void {
    recordingsChannelFilter = value;
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
</script>

<svelte:head>
  <title>Twitch Relay</title>
</svelte:head>

<section class="twitch-panel">
  <AppHeader
    authMode={authController.authMode}
    relayMode="twitch"
    twitchStatus={channelsController.twitchStatus}
    isTwitchStatusLoaded={channelsController.isTwitchStatusLoaded}
    isTwitchBusy={channelsController.isTwitchBusy}
    isBusy={authController.isBusy}
    onToggleMode={() => { /* handled by layout */ }}
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
    {#key viewKey}
      <LoadedFade loaded={true}>
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
            {onLiveOnlyChange}
            onOpenRecordings={openRecordingsOverview}
            onShowAddForm={() => showAddForm = true}
            {cancelAddChannel}
            {submitAddChannel}
            onUpdateNewChannelLogin={(value) => newChannelLogin = value}
            {openChannelSetup}
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
            {recordingsChannelFilter}
            deletingRecordingKey={recordingsController.deletingRecordingKey}
            pinningRecordingKey={recordingsController.pinningRecordingKey}
            repairingRecordingKey={recordingsController.repairingRecordingKey}
            mergingRecordingKey={recordingsController.mergingRecordingKey}
            selectedIncompleteFilenames={recordingsController.selectedIncompleteFilenames}
            pendingJob={recordingsController.pendingJob}
            onBackToChannels={() => backToChannels()}
            onUpdateFilter={(v) => onUpdateRecordingsFilter(v)}
            onOpenRecordingPlayer={(f) => openRecordingPlayer(f)}
            onRemoveRecordingFile={recordingsController.removeRecordingFile}
            onToggleRecordingPin={recordingsController.toggleRecordingPin}
            onRepairRecording={recordingsController.repairRecording}
            onToggleIncompleteMergeSelection={recordingsController.toggleIncompleteMergeSelection}
            onProcessIncompleteFiles={recordingsController.processSelectedIncompleteFiles}
          />
        {/if}
      </LoadedFade>
    {/key}
  {/if}
</section>

<ConfirmRemoveDialog
  channelLogin={confirmRemoveChannel}
  isRemoving={channelsController.isRemovingChannel}
  onConfirm={confirmRemove}
  onCancel={cancelRemove}
/>

<style>
  .twitch-panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, color-mix(in srgb, var(--surface) 95%, transparent), color-mix(in srgb, var(--bg-soft) 95%, transparent));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .ui-error {
    margin: 0 0 0.75rem;
    padding: 0.75rem;
    background: rgba(255, 82, 82, 0.15);
    border: 1px solid var(--danger);
    border-radius: 0.5rem;
    color: var(--danger);
  }

  .ui-muted {
    margin: 0;
    color: var(--muted);
  }
</style>
