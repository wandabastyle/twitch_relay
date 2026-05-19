<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { navigate } from '$lib/router/router.svelte';

  import AppHeader from '$lib/components/home/AppHeader.svelte';
  import AuthPanel from '$lib/components/home/AuthPanel.svelte';
  import TwitchChannelsView from '$lib/components/home/TwitchChannelsView.svelte';
  import TwitchPanel from '$lib/components/twitch/TwitchPanel.svelte';
  import { ConfirmDialog } from '$lib/components/ui';

  import { createAuthController } from '$lib/home/authController.svelte';
  import { createQrController } from '$lib/home/qrController.svelte';
  import { createChannelsController } from '$lib/home/channelsController.svelte';
  import { createRecordingsController } from '$lib/home/recordingsController.svelte';
  import LoadedFade from '$lib/components/LoadedFade.svelte';

  import { loadLiveOnlyPreference, saveLiveOnlyPreference } from '$lib/home/preferences';

  // Global error state (shared across controllers)
  let errorMessage = $state<string | null>(null);

  function setError(message: string | null): void {
    errorMessage = message;
  }

  // Simple UI state (kept in component)
  let showAddForm = $state(false);
  let newChannelLogin = $state('');
  let confirmRemoveChannel = $state<string | null>(null);
  let liveOnly = $state(false);

  // Polling interval reference
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  // Controllers
  const recordingsController = createRecordingsController({ setError });

  const channelsController = createChannelsController({
    setError,
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
    },
  });

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
    navigate('/twitch/recordings');
  }

  function openChannelSetup(channelLogin: string): void {
    navigate(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
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
        bind:liveOnly
        {showAddForm}
        {newChannelLogin}
        isAddingChannel={channelsController.isAddingChannel}
        watchingChannel={channelsController.watchingChannel}
        recordingRules={recordingsController.recordingRules}
        activeRecordings={recordingsController.activeRecordings}
        liveStatusError={channelsController.liveStatusError}
        isLiveStatusLoaded={channelsController.isLiveStatusLoaded}
        {onLiveOnlyChange}
        onOpenRecordings={openRecordingsOverview}
        onShowAddForm={() => showAddForm = true}
        onCancelAddForm={cancelAddChannel}
        onSubmitAddChannel={submitAddChannel}
        onUpdateNewChannelLogin={(value) => newChannelLogin = value}
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
