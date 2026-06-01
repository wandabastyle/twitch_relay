import { useCallback } from 'react';
import type { ChannelsController } from '../hooks/use-channels-controller';
import { navigate } from '../router';

export interface TwitchHomeEventHandlers {
  openRecordingsOverview: () => void;
  openChannelSetup: (channelLogin: string) => void;
  promptRemoveChannel: (login: string) => void;
  confirmRemove: () => void;
  cancelRemove: () => void;
  submitAddChannel: (event: React.SyntheticEvent<HTMLFormElement>) => void;
  cancelAddChannel: () => void;
}

export interface TwitchHomeEventDeps {
  channelsController: ChannelsController;
  newChannelLogin: string;
  confirmRemoveChannel: string | null;
  setConfirmRemoveChannel: (login: string | null) => void;
  setShowAddForm: (show: boolean) => void;
  setNewChannelLogin: (login: string) => void;
  setError: (message: string | null) => void;
}

export const useTwitchHomeEvents = (deps: TwitchHomeEventDeps): TwitchHomeEventHandlers => {
  const {
    channelsController,
    newChannelLogin,
    confirmRemoveChannel,
    setConfirmRemoveChannel,
    setShowAddForm,
    setNewChannelLogin,
    setError,
  } = deps;

  const openRecordingsOverview = useCallback(() => {
    navigate('/twitch/recordings');
  }, []);

  const openChannelSetup = useCallback((channelLogin: string) => {
    navigate(`/twitch/channels/${encodeURIComponent(channelLogin)}`);
  }, []);

  const promptRemoveChannel = useCallback(
    (login: string) => {
      setConfirmRemoveChannel(login);
    },
    [setConfirmRemoveChannel],
  );

  const confirmRemove = useCallback(() => {
    if (confirmRemoveChannel === null || confirmRemoveChannel === '') {
      return;
    }
    void channelsController.confirmRemoveChannel(confirmRemoveChannel);
    setConfirmRemoveChannel(null);
  }, [confirmRemoveChannel, channelsController, setConfirmRemoveChannel]);

  const cancelRemove = useCallback(() => {
    setConfirmRemoveChannel(null);
  }, [setConfirmRemoveChannel]);

  const submitAddChannel = useCallback(
    (event: React.SyntheticEvent<HTMLFormElement>): void => {
      event.preventDefault();
      void channelsController.submitAddChannel(newChannelLogin);
      setNewChannelLogin('');
      setShowAddForm(false);
    },
    [channelsController, newChannelLogin, setNewChannelLogin, setShowAddForm],
  );

  const cancelAddChannel = useCallback(() => {
    setShowAddForm(false);
    setNewChannelLogin('');
    setError(null);
  }, [setShowAddForm, setNewChannelLogin, setError]);

  return {
    cancelAddChannel,
    cancelRemove,
    confirmRemove,
    openChannelSetup,
    openRecordingsOverview,
    promptRemoveChannel,
    submitAddChannel,
  };
};
