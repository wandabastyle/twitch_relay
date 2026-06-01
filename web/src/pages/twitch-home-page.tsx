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

interface TwitchHomeState {
  confirmRemoveChannel: string | null;
  errorMessage: string | null;
  isInitialLoadComplete: boolean;
  newChannelLogin: string;
  setConfirmRemoveChannel: (value: string | null) => void;
  setErrorMessage: (value: string | null) => void;
  setIsInitialLoadComplete: (value: boolean) => void;
  setNewChannelLogin: (value: string) => void;
  setShowAddForm: (value: boolean) => void;
  showAddForm: boolean;
}

const useTwitchHomeState = (): TwitchHomeState => {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newChannelLogin, setNewChannelLogin] = useState('');
  const [confirmRemoveChannel, setConfirmRemoveChannel] = useState<string | null>(null);
  const [isInitialLoadComplete, setIsInitialLoadComplete] = useState(false);

  return {
    confirmRemoveChannel,
    errorMessage,
    isInitialLoadComplete,
    newChannelLogin,
    setConfirmRemoveChannel,
    setErrorMessage,
    setIsInitialLoadComplete,
    setNewChannelLogin,
    setShowAddForm,
    showAddForm,
  };
};

interface TwitchHomeContentProps {
  authController: ReturnType<typeof useAuthController>;
  channelsController: ReturnType<typeof useChannelsController>;
  eventHandlers: ReturnType<typeof useTwitchHomeEvents>;
  qrController: ReturnType<typeof useQrController>;
  recordingsController: ReturnType<typeof useRecordingsController>;
  state: TwitchHomeState;
}

const TwitchHomeContent = (props: TwitchHomeContentProps): ReactElement => {
  const {
    authController,
    channelsController,
    eventHandlers,
    qrController,
    recordingsController,
    state,
  } = props;

  if (authController.authMode === 'unauthenticated') {
    return (
      <AuthPanel
        loginMode={qrController.loginMode}
        accessCode={authController.accessCode}
        qrDataUrl={qrController.qrDataUrl}
        isBusy={authController.isBusy}
        onSubmitLogin={(event) => {
          void authController.submitLogin(event);
        }}
        onSwitchToQr={() => {
          void qrController.switchToQrMode();
        }}
        onSwitchToCode={() => {
          qrController.switchToCodeMode();
        }}
        onUpdateAccessCode={authController.setAccessCode}
      />
    );
  }

  return (
    <LoadedFade loaded={true}>
      <TwitchChannelsView
        channels={channelsController.channels}
        liveStatus={channelsController.liveStatus}
        showAddForm={state.showAddForm}
        newChannelLogin={state.newChannelLogin}
        isAddingChannel={channelsController.isAddingChannel}
        watchingChannel={channelsController.watchingChannel ?? undefined}
        recordingRules={recordingsController.recordingRules}
        activeRecordings={recordingsController.activeRecordings}
        liveStatusError={channelsController.liveStatusError ?? undefined}
        isLiveStatusLoaded={channelsController.isLiveStatusLoaded}
        onOpenRecordings={eventHandlers.openRecordingsOverview}
        onShowAddForm={() => {
          state.setShowAddForm(true);
        }}
        onCancelAddForm={eventHandlers.cancelAddChannel}
        onSubmitAddChannel={eventHandlers.submitAddChannel}
        onUpdateNewChannelLogin={(value) => {
          state.setNewChannelLogin(value);
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
  );
};

const RemoveChannelDialog = ({
  channelsController,
  eventHandlers,
  state,
}: Pick<
  TwitchHomeContentProps,
  'channelsController' | 'eventHandlers' | 'state'
>): ReactElement => (
  <ConfirmDialog
    isOpen={state.confirmRemoveChannel !== null}
    isBusy={channelsController.isRemovingChannel}
    onConfirm={eventHandlers.confirmRemove}
    onCancel={eventHandlers.cancelRemove}
    confirmText={channelsController.isRemovingChannel ? 'Removing...' : 'Remove'}
    confirmVariant="danger"
  >
    <p>
      Remove <strong className="danger-text">{state.confirmRemoveChannel}</strong> from the channel
      list?
    </p>
  </ConfirmDialog>
);

export const TwitchHomePage = (): ReactElement => {
  const state = useTwitchHomeState();
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const setError = useCallback(
    (message: string | null): void => {
      state.setErrorMessage(message);
    },
    [state],
  );

  const recordingsController = useRecordingsController({ setError });

  const channelsController = useChannelsController({
    onChannelsLoaded: async () => {
      await recordingsController.loadRecordingState();
      await recordingsController.loadRecordingRules();
      state.setIsInitialLoadComplete(true);
    },
    setError,
  });

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

  const eventHandlers = useTwitchHomeEvents({
    channelsController,
    confirmRemoveChannel: state.confirmRemoveChannel,
    newChannelLogin: state.newChannelLogin,
    setConfirmRemoveChannel: state.setConfirmRemoveChannel,
    setError,
    setNewChannelLogin: state.setNewChannelLogin,
    setShowAddForm: state.setShowAddForm,
  });

  useEffect((): (() => void) | undefined => {
    if (state.isInitialLoadComplete) {
      startPolling();
      return (): void => {
        stopPolling();
      };
    }
    return undefined;
  }, [state.isInitialLoadComplete, startPolling, stopPolling]);

  useEffect(() => {
    void authController.initialize();
  }, []);

  useEffect(
    (): (() => void) => () => {
      stopPolling();
      qrController.cleanup();
    },
    [stopPolling, qrController],
  );

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

      {state.errorMessage !== null && state.errorMessage !== '' && (
        <p className="ui-error" role="alert">
          {state.errorMessage}
        </p>
      )}

      <TwitchHomeContent
        authController={authController}
        channelsController={channelsController}
        eventHandlers={eventHandlers}
        qrController={qrController}
        recordingsController={recordingsController}
        state={state}
      />

      <RemoveChannelDialog
        channelsController={channelsController}
        eventHandlers={eventHandlers}
        state={state}
      />
    </TwitchPanel>
  );
};
