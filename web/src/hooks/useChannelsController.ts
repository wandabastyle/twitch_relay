import { useState, useCallback } from 'react';
import {
  addChannel,
  createWatchTicket,
  disconnectTwitch,
  getChannels,
  getLiveStatus,
  getTwitchConnectUrl,
  getTwitchStatus,
  removeChannel,
} from '../api-client';
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '../api-client/types';
import { navigate } from '../router';
import {
  handleChannelsLoadError,
  handleChannelsLoadSuccess,
  handleLiveStatusError,
  handleLiveStatusSuccess,
  handleTwitchStatusError,
  handleTwitchStatusSuccess,
  loadCachedTwitchStatus,
  saveCachedTwitchStatus,
  clearCachedTwitchStatus,
  createInitialTwitchStatus,
  validateChannelLogin,
} from './channels-controller-helpers';

const EMPTY_ARRAY_LENGTH = 0;
const EMPTY_OBJECT_KEYS_LENGTH = 0;

export interface ChannelsControllerDeps {
  onChannelsLoaded?: () => Promise<void>;
  setError: (message: string | null) => void;
}

export interface ChannelsController {
  channels: ChannelEntry[];
  confirmRemoveChannel: (login: string) => Promise<void>;
  connectTwitch: () => void;
  isAddingChannel: boolean;
  isChannelsLoaded: boolean;
  isLiveStatusLoaded: boolean;
  isRemovingChannel: boolean;
  isTwitchBusy: boolean;
  isTwitchStatusLoaded: boolean;
  liveStatus: Record<string, ChannelStatus>;
  liveStatusError: string | null;
  loadChannels: () => Promise<void>;
  loadLiveStatus: () => Promise<void>;
  resetState: () => void;
  startWatching: (channelLogin: string) => Promise<void>;
  submitAddChannel: (newChannelLogin: string) => Promise<void>;
  twitchStatus: TwitchStatusResponse;
  unlinkTwitch: () => Promise<void>;
  watchingChannel: string | null;
}

export const useChannelsController = (deps: ChannelsControllerDeps): ChannelsController => {
  const cachedStatus = loadCachedTwitchStatus();
  const initialStatus = createInitialTwitchStatus(cachedStatus);

  const [channels, setChannels] = useState<ChannelEntry[]>([]);
  const [isChannelsLoaded, setIsChannelsLoaded] = useState(false);
  const [liveStatus, setLiveStatus] = useState<Record<string, ChannelStatus>>({});
  const [isLiveStatusLoaded, setIsLiveStatusLoaded] = useState(false);
  const [liveStatusError, setLiveStatusError] = useState<string | null>(null);
  const [twitchStatus, setTwitchStatus] = useState<TwitchStatusResponse>(initialStatus);
  const [isTwitchStatusLoaded, setIsTwitchStatusLoaded] = useState(cachedStatus !== null);
  const [isTwitchBusy, setIsTwitchBusy] = useState(false);
  const [watchingChannel, setWatchingChannel] = useState<string | null>(null);
  const [isAddingChannel, setIsAddingChannel] = useState(false);
  const [isRemovingChannel, setIsRemovingChannel] = useState(false);

  const loadTwitchStatus = useCallback(async (): Promise<void> => {
    try {
      const newStatus = await getTwitchStatus();
      handleTwitchStatusSuccess(newStatus, {
        setError: deps.setError,
        setLoaded: setIsTwitchStatusLoaded,
        setStatus: setTwitchStatus,
      });
    } catch (error) {
      handleTwitchStatusError(
        isTwitchStatusLoaded,
        {
          setError: deps.setError,
          setLoaded: setIsTwitchStatusLoaded,
          setStatus: setTwitchStatus,
        },
        error,
      );
    }
  }, [deps, isTwitchStatusLoaded]);

  const loadLiveStatusInternal = useCallback(async (): Promise<void> => {
    try {
      const status = await getLiveStatus();
      handleLiveStatusSuccess(status.channels, {
        setError: setLiveStatusError,
        setLoaded: setIsLiveStatusLoaded,
        setStatus: setLiveStatus,
      });
    } catch {
      handleLiveStatusError({
        setError: setLiveStatusError,
        setLoaded: setIsLiveStatusLoaded,
        setStatus: setLiveStatus,
      });
    }
  }, []);

  const loadChannelsInternal = useCallback(async (): Promise<void> => {
    const twitchPromise = loadTwitchStatus();
    try {
      const newChannels = await getChannels();
      handleChannelsLoadSuccess(newChannels, {
        onChannelsLoaded: deps.onChannelsLoaded,
        setChannels,
        setError: deps.setError,
        setLoaded: setIsChannelsLoaded,
      });
      await loadLiveStatusInternal();
    } catch (error) {
      handleChannelsLoadError(
        channels,
        {
          onChannelsLoaded: deps.onChannelsLoaded,
          setChannels,
          setError: deps.setError,
          setLoaded: setIsChannelsLoaded,
        },
        error,
      );
    }
    await twitchPromise;
  }, [channels, deps, loadTwitchStatus, loadLiveStatusInternal]);

  const submitAddChannel = useCallback(
    async (newChannelLogin: string): Promise<void> => {
      const normalized = validateChannelLogin(newChannelLogin);
      if (normalized === null) {
        deps.setError('channel name is required');
        return;
      }
      setIsAddingChannel(true);
      deps.setError(null);
      try {
        await addChannel(normalized);
        await loadChannelsInternal();
      } catch (error) {
        deps.setError(error instanceof Error ? error.message : String(error));
      } finally {
        setIsAddingChannel(false);
      }
    },
    [deps, loadChannelsInternal],
  );

  const confirmRemoveChannel = useCallback(
    async (login: string): Promise<void> => {
      setIsRemovingChannel(true);
      deps.setError(null);
      try {
        await removeChannel(login);
        await loadChannelsInternal();
      } catch (error) {
        deps.setError(error instanceof Error ? error.message : String(error));
      } finally {
        setIsRemovingChannel(false);
      }
    },
    [deps, loadChannelsInternal],
  );

  const connectTwitch = useCallback((): void => {
    window.location.assign(getTwitchConnectUrl());
  }, []);

  const unlinkTwitch = useCallback(async (): Promise<void> => {
    setIsTwitchBusy(true);
    deps.setError(null);
    try {
      await disconnectTwitch();
      setTwitchStatus({ connected: false, scopes: [] });
      setIsTwitchStatusLoaded(true);
      saveCachedTwitchStatus({ connected: false, scopes: [] });
      await loadChannelsInternal();
    } catch (error) {
      deps.setError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsTwitchBusy(false);
    }
  }, [deps, loadChannelsInternal]);

  const startWatching = useCallback(
    async (channelLogin: string): Promise<void> => {
      setWatchingChannel(channelLogin);
      deps.setError(null);
      try {
        const ticket = await createWatchTicket(channelLogin);
        navigate(`/watch/${ticket.watch_url}`);
      } catch (error) {
        deps.setError(error instanceof Error ? error.message : String(error));
        setWatchingChannel(null);
      }
    },
    [deps],
  );

  const resetState = useCallback((): void => {
    setChannels([]);
    setIsChannelsLoaded(false);
    setLiveStatus({});
    setIsLiveStatusLoaded(false);
    setLiveStatusError(null);
    setTwitchStatus({ connected: false, scopes: [] });
    setIsTwitchStatusLoaded(false);
    setIsTwitchBusy(false);
    setWatchingChannel(null);
    setIsAddingChannel(false);
    setIsRemovingChannel(false);
    clearCachedTwitchStatus();
  }, []);

  return {
    channels,
    confirmRemoveChannel,
    connectTwitch,
    isAddingChannel,
    isChannelsLoaded,
    isLiveStatusLoaded,
    isRemovingChannel,
    isTwitchBusy,
    isTwitchStatusLoaded,
    liveStatus,
    liveStatusError,
    loadChannels: loadChannelsInternal,
    loadLiveStatus: loadLiveStatusInternal,
    resetState,
    startWatching,
    submitAddChannel,
    twitchStatus,
    unlinkTwitch,
    watchingChannel,
  };
}
