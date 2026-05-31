import { useCallback, useEffect, useRef, useState, type ReactElement } from 'react';
import { AppHeader } from '../components/shared/app-header';
import { AuthPanel } from '../components/twitch/auth-panel';
import { TwitchChannelsView } from '../components/twitch/twitch-channels-view';
import { TwitchPanel } from '../components/twitch/twitch-panel';
import { ConfirmDialog } from '../components/ui/confirm-dialog';
import { LoadedFade } from '../components/ui/loaded-fade';
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
  const [isInitialLoadComplete, setIsInitialLoadComplete] = useState(false);

  // Polling interval reference - stored in ref to avoid recreating callbacks
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Initialize controllers
  const recordingsController = useRecordingsController({ setError });

  const channelsController = useChannelsController({
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
      setIsInitialLoadComplete(true);
    },
    setError,
  });

  // Stable polling function using refs
  const startPolling = useCallback((): void => {
    // Clear any existing interval first
    if (pollIntervalRef.current !== null) {
      clearInterval(pollIntervalRef.current);
    }
    pollIntervalRef.current = setInterval(() => {
      void channelsController.loadLiveStatus();
      void recordingsController.loadRecordingState();
    }, POLL_INTERVAL_MS);
  }, [channelsController, recordingsController]);

  const stopPolling = useCallback((): void => {
    if (pollIntervalRef.current !== null) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  const authController = useAuthController({
    channelsController,
    onAuthenticated: async () => {
      await channelsController.loadChannels();
      // Start polling only after channels are loaded
      startPolling();
    },
    setError,
  });

  const qrController = useQrController({
    onQrAuthenticated: () => {
      globalThis.location.reload();
    },
    setError,
  });

  // Handle polling based on initial load completion - no unstable dependencies
  useEffect((): (() => void) | undefined => {
    if (isInitialLoadComplete) {
      startPolling();
      return (): void => {
        stopPolling();
      };
    }
    return undefined;
  }, [isInitialLoadComplete, startPolling, stopPolling]);

  // Initialize on mount
  useEffect(() => {
    void authController.initialize();
  }, []);

  // Cleanup on unmount
  useEffect((): (() => void) => (): void => {
    stopPolling();
    qrController.cleanup();
  }, [stopPolling, qrController]);

  const openRecordingsOverview = useCallback(() => {
    navigate('/twitch/recordings');
  }, []);

  const openChannelSetup = useCallback((channelLogin: string) => {
    navigate(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
  }, []);

  const promptRemoveChannel = useCallback((login: string) => {
    setConfirmRemoveChannel(login);
  }, []);

  const confirmRemove = useCallback(() => {
    if (confirmRemoveChannel === null || confirmRemoveChannel === '') {
      return;
    }
    void channelsController.confirmRemoveChannel(confirmRemoveChannel);
    setConfirmRemoveChannel(null);
  }, [confirmRemoveChannel, channelsController]);

  const cancelRemove = useCallback(() => {
    setConfirmRemoveChannel(null);
  }, []);

  const submitAddChannel = useCallback(
    (event: React.SyntheticEvent<HTMLFormElement>): void => {
      event.preventDefault();
      void channelsController.submitAddChannel(newChannelLogin);
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
        onToggleMode={() => {
          navigate('/youtube');
        }}
        onConnectTwitch={() => {
          channelsController.connectTwitch();
        }}
        onDisconnectTwitch={() => {
          void channelsController.unlinkTwitch();
        }}
        onSignOut={() => {
          void authController.signOut();
        }}
      />

      {errorMessage !== null && errorMessage !== '' && (
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
          onSubmitLogin={(event) => {
            void authController.submitLogin(event);
          }}
          onSwitchToQr={() => {
            void (async (): Promise<void> => {
              await qrController.switchToQrMode();
            })();
          }}
          onSwitchToCode={() => {
            qrController.switchToCodeMode();
          }}
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
            onShowAddForm={() => {
              setShowAddForm(true);
            }}
            onCancelAddForm={cancelAddChannel}
            onSubmitAddChannel={submitAddChannel}
            onUpdateNewChannelLogin={(value) => {
              setNewChannelLogin(value);
            }}
            onOpenChannelSetup={openChannelSetup}
            onStartWatching={(login) => {
              void channelsController.startWatching(login);
            }}
            onToggleAutoRecord={(login) => {
              void recordingsController.toggleAutoRecord(login);
            }}
            onToggleManualRecording={(login) => {
              void recordingsController.toggleManualRecording(
                login,
                recordingsController.selectedQuality(login),
                channelsController.liveStatus[login]?.title,
              );
            }}
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
