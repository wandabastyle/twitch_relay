<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';

  import AppHeader from '$lib/components/home/AppHeader.svelte';
  import AuthPanel from '$lib/components/home/AuthPanel.svelte';
  import TwitchChannelsView from '$lib/components/home/TwitchChannelsView.svelte';
  import ConfirmRemoveDialog from '$lib/components/home/ConfirmRemoveDialog.svelte';
  import TwitchPanel from '$lib/components/twitch/TwitchPanel.svelte';

  import { createAuthController } from '$lib/home/authController.svelte';
  import { createQrController } from '$lib/home/qrController.svelte';
  import { createChannelsController } from '$lib/home/channelsController.svelte';
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
  const channelsController = createChannelsController({
    setError,
    onChannelsLoaded: async () => {
      // No-op - recordings page is separate now
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
    }, 60000);
  }

  function onLiveOnlyChange(_value: boolean): void {
    saveLiveOnlyPreference(liveOnly);
  }

  function openRecordingsOverview(): void {
    goto('/twitch/recordings');
  }

  function openChannelSetup(channelLogin: string): void {
    goto(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
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

<svelte:head>
  <title>Twitch Relay</title>
</svelte:head>

<TwitchPanel>
  <AppHeader
    authMode={authController.authMode}
    relayMode="twitch"
    twitchStatus={channelsController.twitchStatus}
    isTwitchStatusLoaded={channelsController.isTwitchStatusLoaded}
    isTwitchBusy={channelsController.isTwitchBusy}
    isBusy={authController.isBusy}
    onToggleMode={() => goto('/youtube')}
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
    <LoadedFade loaded={true}>
      <TwitchChannelsView
        channels={channelsController.channels}
        liveStatus={channelsController.liveStatus}
        bind:liveOnly
        {showAddForm}
        {newChannelLogin}
        isAddingChannel={channelsController.isAddingChannel}
        watchingChannel={channelsController.watchingChannel}
        recordingRules={{}}
        activeRecordings={{}}
        liveStatusError={channelsController.liveStatusError}
        {onLiveOnlyChange}
        onOpenRecordings={openRecordingsOverview}
        onShowAddForm={() => showAddForm = true}
        onCancelAddForm={cancelAddChannel}
        onSubmitAddChannel={submitAddChannel}
        onUpdateNewChannelLogin={(value) => newChannelLogin = value}
        onOpenChannelSetup={openChannelSetup}
        onStartWatching={channelsController.startWatching}
        onToggleAutoRecord={() => {}}
        onToggleManualRecording={() => {}}
        onPromptRemoveChannel={promptRemoveChannel}
      />
    </LoadedFade>
  {/if}
</TwitchPanel>

<ConfirmRemoveDialog
  channelLogin={confirmRemoveChannel}
  isRemoving={channelsController.isRemovingChannel}
  onConfirm={confirmRemove}
  onCancel={cancelRemove}
/>

<style>
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
