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
import { useTwitchHomeEvents } from './twitch-home-events';

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

  // Polling interval reference
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

  // Event handlers from extracted module
  const eventHandlers = useTwitchHomeEvents({
    channelsController,
    confirmRemoveChannel,
    newChannelLogin,
    setConfirmRemoveChannel,
    setError,
    setNewChannelLogin,
    setShowAddForm,
  });

  // Handle polling based on initial load completion
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
            onOpenRecordings={eventHandlers.openRecordingsOverview}
            onShowAddForm={() => {
              setShowAddForm(true);
            }}
            onCancelAddForm={eventHandlers.cancelAddChannel}
            onSubmitAddChannel={eventHandlers.submitAddChannel}
            onUpdateNewChannelLogin={(value) => {
              setNewChannelLogin(value);
            }}
            onOpenChannelSetup={eventHandlers.openChannelSetup}
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
            onPromptRemoveChannel={eventHandlers.promptRemoveChannel}
          />
        </LoadedFade>
      )}

      <ConfirmDialog
        isOpen={confirmRemoveChannel !== null}
        isBusy={channelsController.isRemovingChannel}
        onConfirm={eventHandlers.confirmRemove}
        onCancel={eventHandlers.cancelRemove}
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
};
