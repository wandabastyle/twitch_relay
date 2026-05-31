import { useCallback, useEffect, useState, type FormEvent } from 'react';
import type { ReactElement } from 'react';
import { AppHeader } from '../components/shared/AppHeader';
import { AuthPanel } from '../components/twitch/AuthPanel';
import { TwitchChannelsView } from '../components/twitch/TwitchChannelsView';
import { TwitchPanel } from '../components/twitch/TwitchPanel';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { LoadedFade } from '../components/ui/LoadedFade';
import {
  useAuthController,
  useChannelsController,
  useQrController,
  useRecordingsController,
} from '../hooks';
import { navigate } from '../router';

const POLL_INTERVAL_MS = 60_000;

export const TwitchHomePage = (): ReactElement => {
  // Global error state (shared across controllers)
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const setError = useCallback((message: string | null): void => {
    setErrorMessage(message);
  }, []);

  // Simple UI state (kept in component)
  const [showAddForm, setShowAddForm] = useState(false);
  const [newChannelLogin, setNewChannelLogin] = useState('');
  const [confirmRemoveChannel, setConfirmRemoveChannel] = useState<string | null>(null);

  // Initialize controllers
  const recordingsController = useRecordingsController({ setError });

  const channelsController = useChannelsController({
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
    },
    setError,
  });

  const startPolling = useCallback(() => {
    return setInterval(async () => {
      await channelsController.loadLiveStatus();
      await recordingsController.loadRecordingState();
    }, POLL_INTERVAL_MS);
  }, [channelsController, recordingsController]);

  const authController = useAuthController({
    channelsController,
    onAuthenticated: async () => {
      await channelsController.loadChannels();
      // Start polling is handled in useEffect when auth mode changes
    },
    setError,
  });

  const qrController = useQrController({
    onQrAuthenticated: () => {
      window.location.reload();
    },
    setError,
  });

  // Handle polling effect
  useEffect(() => {
    if (authController.authMode === 'authenticated') {
      const interval = startPolling();
      return () => {
        clearInterval(interval);
      };
    }
  }, [authController.authMode, startPolling]);

  // Initialize on mount
  useEffect(() => {
    void authController.initialize();
  }, []);

  // Cleanup QR controller on unmount
  useEffect(() => {
    return () => {
      qrController.cleanup();
    };
  }, [qrController]);

  const openRecordingsOverview = useCallback(() => {
    navigate('/twitch/recordings');
  }, []);

  const openChannelSetup = useCallback((channelLogin: string) => {
    navigate(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
  }, []);

  const promptRemoveChannel = useCallback((login: string) => {
    setConfirmRemoveChannel(login);
  }, []);

  const confirmRemove = useCallback(async () => {
    if (!confirmRemoveChannel) {
      return;
    }
    await channelsController.confirmRemoveChannel(confirmRemoveChannel);
    setConfirmRemoveChannel(null);
  }, [confirmRemoveChannel, channelsController]);

  const cancelRemove = useCallback(() => {
    setConfirmRemoveChannel(null);
  }, []);

  const submitAddChannel = useCallback(
    async (event: FormEvent) => {
      event.preventDefault();
      await channelsController.submitAddChannel(newChannelLogin);
      setNewChannelLogin('');
      setShowAddForm(false);
    },
    [channelsController, newChannelLogin],
  );

  const cancelAddChannel = useCallback(() => {
    setShowAddForm(false);
    setNewChannelLogin('');
    setError(null);
  }, [setError]);

  return (
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

      {errorMessage && (
        <p className="ui-error" role="alert">
          {errorMessage}
        </p>
      )}

      {authController.authMode === 'unauthenticated' ? (
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
      ) : (
        <LoadedFade loaded={true}>
          <TwitchChannelsView
            channels={channelsController.channels}
            liveStatus={channelsController.liveStatus}
            showAddForm={showAddForm}
            newChannelLogin={newChannelLogin}
            isAddingChannel={channelsController.isAddingChannel}
            watchingChannel={channelsController.watchingChannel ?? undefined}
            recordingRules={recordingsController.recordingRules}
            activeRecordings={recordingsController.activeRecordings}
            liveStatusError={channelsController.liveStatusError ?? undefined}
            isLiveStatusLoaded={channelsController.isLiveStatusLoaded}
            onOpenRecordings={openRecordingsOverview}
            onShowAddForm={() => setShowAddForm(true)}
            onCancelAddForm={cancelAddChannel}
            onSubmitAddChannel={submitAddChannel}
            onUpdateNewChannelLogin={(value) => setNewChannelLogin(value)}
            onOpenChannelSetup={openChannelSetup}
            onStartWatching={channelsController.startWatching}
            onToggleAutoRecord={recordingsController.toggleAutoRecord}
            onToggleManualRecording={(login) =>
              recordingsController.toggleManualRecording(
                login,
                recordingsController.selectedQuality(login),
                channelsController.liveStatus[login]?.title,
              )
            }
            onPromptRemoveChannel={promptRemoveChannel}
          />
        </LoadedFade>
      )}

      <ConfirmDialog
        isOpen={confirmRemoveChannel !== null}
        isBusy={channelsController.isRemovingChannel}
        onConfirm={confirmRemove}
        onCancel={cancelRemove}
        confirmText={channelsController.isRemovingChannel ? 'Removing...' : 'Remove'}
        confirmVariant="danger"
      >
        <p>
          Remove <strong className="danger-text">{confirmRemoveChannel}</strong> from the channel
          list?
        </p>
      </ConfirmDialog>
    </TwitchPanel>
  );
}
