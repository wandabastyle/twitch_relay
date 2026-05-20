<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { createAuthController } from '$lib/home/auth-controller.svelte';
  import { createChannelsController } from '$lib/home/channels-controller.svelte';
  import { createQrController } from '$lib/home/qr-controller.svelte';
  import { createRecordingsController } from '$lib/home/recordings-controller.svelte';
  import { loadLiveOnlyPreference, saveLiveOnlyPreference } from '$lib/home/preferences';
  import { navigate } from '$lib/router/router.svelte';
  import { ConfirmDialog } from '$lib/components/ui';
  import LoadedFade from '$lib/components/loaded-fade.svelte';
  import AppHeader from '$lib/components/home/app-header.svelte';
  import AuthPanel from '$lib/components/home/auth-panel.svelte';
  import TwitchChannelsView from '$lib/components/home/twitch-channels-view.svelte';
  import TwitchPanel from '$lib/components/twitch/twitch-panel.svelte';

  const POLL_INTERVAL_MS = 60_000;

  // Global error state (shared across controllers)
  let errorMessage = $state<string | null>(null);

  const setError = (message: string | null): void => {
    errorMessage = message;
  };

  // Simple UI state (kept in component)
  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let confirmRemoveChannel = $state<string | null>(null);
  let liveOnly = $state(loadLiveOnlyPreference());

  // Polling interval reference
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  // Helper functions
  const stopPolling = (): void => {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  };

  // Initialize controllers
  const recordingsController = createRecordingsController({ setError });

  const channelsController = createChannelsController({
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
    },
    setError,
  });

  const startPolling = (): void => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
    pollInterval = setInterval(async () => {
      await channelsController.loadLiveStatus();
      await recordingsController.loadRecordingState();
    }, POLL_INTERVAL_MS);
  };

  const authController = createAuthController({
    onAuthenticated: async () => {
      await channelsController.loadChannels();
      startPolling();
    },
    setError,
  });

  const qrController = createQrController({
    onQrAuthenticated: () => {
      globalThis.location.reload();
    },
    setError,
  });

  const onLiveOnlyChange = (value: boolean): void => {
    liveOnly = value;
    saveLiveOnlyPreference(value);
  };

  const openRecordingsOverview = (): void => {
    navigate('/twitch/recordings');
  };

  const openChannelSetup = (channelLogin: string): void => {
    navigate(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
  };

  const promptRemoveChannel = (login: string): void => {
    confirmRemoveChannel = login;
  };

  const confirmRemove = async (): Promise<void> => {
    if (!confirmRemoveChannel) {
      return;
    }
    await channelsController.confirmRemoveChannel(confirmRemoveChannel);
    confirmRemoveChannel = null;
  };

  const cancelRemove = (): void => {
    confirmRemoveChannel = null;
  };

  const submitAddChannel = async (event: SubmitEvent): Promise<void> => {
    event.preventDefault();
    await channelsController.submitAddChannel(newChannelLogin);
    newChannelLogin = '';
    showAddForm = false;
  };

  const cancelAddChannel = (): void => {
    showAddForm = false;
    newChannelLogin = '';
    setError(null);
  };

  onMount(async () => {
    await authController.initialize();
  });

  onDestroy(() => {
    stopPolling();
    qrController.cleanup();
  });
</script>

<TwitchPanel>
  <AppHeader
    authMode={authController.authMode}
    relayMode="twitch"
    twitchStatus={channelsController.twitchStatus}
    isTwitchStatusLoaded={channelsController.isTwitchStatusLoaded}
    isTwitchBusy={channelsController.isTwitchBusy}
    isBusy={authController.isBusy}
    onToggleMode={() => navigate('/youtube')}
    onConnectTwitch={channelsController.connectTwitch}
    onDisconnectTwitch={channelsController.unlinkTwitch}
    onSignOut={authController.signOut}
  />

  {#if errorMessage}
    <p class="ui-error" role="alert">{errorMessage}</p>
  {/if}

  {#if authController.authMode === 'unauthenticated' && channelsController.channels.length === 0}
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
    <LoadedFade loaded={true}>
      <TwitchChannelsView
        channels={channelsController.channels}
        liveStatus={channelsController.liveStatus}
        {liveOnly}
        {showAddForm}
        {newChannelLogin}
        isAddingChannel={channelsController.isAddingChannel}
        watchingChannel={channelsController.watchingChannel ?? undefined}
        recordingRules={recordingsController.recordingRules}
        activeRecordings={recordingsController.activeRecordings}
        liveStatusError={channelsController.liveStatusError ?? undefined}
        isLiveStatusLoaded={channelsController.isLiveStatusLoaded}
        {onLiveOnlyChange}
        onOpenRecordings={openRecordingsOverview}
        onShowAddForm={() => { showAddForm = true; }}
        onCancelAddForm={cancelAddChannel}
        onSubmitAddChannel={submitAddChannel}
        onUpdateNewChannelLogin={(value) => { newChannelLogin = value; }}
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
    </LoadedFade>
  {/if}
</TwitchPanel>

<ConfirmDialog
  isOpen={confirmRemoveChannel !== null}
  isBusy={channelsController.isRemovingChannel}
  onConfirm={confirmRemove}
  onCancel={cancelRemove}
  confirmText={channelsController.isRemovingChannel ? 'Removing...' : 'Remove'}
  confirmVariant="danger"
>
  <p>
    Remove <strong class="danger-text">{confirmRemoveChannel}</strong> from the channel list?
  </p>
</ConfirmDialog>

<style>
  .ui-error {
    margin: 0 0 0.75rem;
    padding: 0.75rem;
    background: rgba(255, 82, 82, 0.15);
    border: 1px solid var(--danger);
    border-radius: 0.5rem;
    color: var(--danger);
  }
</style>
