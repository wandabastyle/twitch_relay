import { useCallback } from 'react';
import { getChannels, getLiveStatus, getTwitchStatus } from '../api-client';
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '../api-client/types';
import {
  handleChannelsLoadError,
  handleChannelsLoadSuccess,
  handleLiveStatusError,
  handleLiveStatusSuccess,
  handleTwitchStatusError,
  handleTwitchStatusSuccess,
} from './channels-controller-helpers';

export interface TwitchStatusDeps {
  setError: (message: string | null) => void;
  onChannelsLoaded?: () => Promise<void>;
}

export interface TwitchStatusState {
  channels: ChannelEntry[];
  isChannelsLoaded: boolean;
  isLiveStatusLoaded: boolean;
  isTwitchStatusLoaded: boolean;
  liveStatus: Record<string, ChannelStatus>;
  twitchStatus: TwitchStatusResponse;
}

export interface TwitchStatusSetters {
  setChannels: (channels: ChannelEntry[]) => void;
  setIsChannelsLoaded: (loaded: boolean) => void;
  setLiveStatus: (status: Record<string, ChannelStatus>) => void;
  setIsLiveStatusLoaded: (loaded: boolean) => void;
  setLiveStatusError: (error: string | null) => void;
  setTwitchStatus: (status: TwitchStatusResponse) => void;
  setIsTwitchStatusLoaded: (loaded: boolean) => void;
}

export interface TwitchStatusPollerReturn {
  loadTwitchStatus: () => Promise<void>;
  loadLiveStatusInternal: () => Promise<void>;
  loadChannelsInternal: () => Promise<void>;
}

export const createTwitchStatusPoller = (
  state: TwitchStatusState,
  setters: TwitchStatusSetters,
  deps: TwitchStatusDeps,
): TwitchStatusPollerReturn => {
  const loadTwitchStatus = async (): Promise<void> => {
    try {
      const newStatus = await getTwitchStatus();
      handleTwitchStatusSuccess(newStatus, {
        setError: deps.setError,
        setLoaded: setters.setIsTwitchStatusLoaded,
        setStatus: setters.setTwitchStatus,
      });
    } catch (error) {
      handleTwitchStatusError(
        state.isTwitchStatusLoaded,
        {
          setError: deps.setError,
          setLoaded: setters.setIsTwitchStatusLoaded,
          setStatus: setters.setTwitchStatus,
        },
        error,
      );
    }
  };

  const loadLiveStatusInternal = async (): Promise<void> => {
    try {
      const status = await getLiveStatus();
      handleLiveStatusSuccess(status.channels, {
        setError: setters.setLiveStatusError,
        setLoaded: setters.setIsLiveStatusLoaded,
        setStatus: setters.setLiveStatus,
      });
    } catch {
      handleLiveStatusError({
        setError: setters.setLiveStatusError,
        setLoaded: setters.setIsLiveStatusLoaded,
        setStatus: setters.setLiveStatus,
      });
    }
  };

  const loadChannelsInternal = async (): Promise<void> => {
    const twitchPromise = loadTwitchStatus();
    try {
      const newChannels = await getChannels();
      handleChannelsLoadSuccess(newChannels, {
        onChannelsLoaded: deps.onChannelsLoaded,
        setChannels: setters.setChannels,
        setError: deps.setError,
        setLoaded: setters.setIsChannelsLoaded,
      });
      await loadLiveStatusInternal();
    } catch (error) {
      handleChannelsLoadError(
        state.channels,
        {
          onChannelsLoaded: deps.onChannelsLoaded,
          setChannels: setters.setChannels,
          setError: deps.setError,
          setLoaded: setters.setIsChannelsLoaded,
        },
        error,
      );
    }
    await twitchPromise;
  };

  return {
    loadChannelsInternal,
    loadLiveStatusInternal,
    loadTwitchStatus,
  };
};

export const useTwitchStatusPoller = (
  state: TwitchStatusState,
  setters: TwitchStatusSetters,
  deps: TwitchStatusDeps,
): TwitchStatusPollerReturn => {
  const loadTwitchStatus = useCallback(async (): Promise<void> => {
    try {
      const newStatus = await getTwitchStatus();
      handleTwitchStatusSuccess(newStatus, {
        setError: deps.setError,
        setLoaded: setters.setIsTwitchStatusLoaded,
        setStatus: setters.setTwitchStatus,
      });
    } catch (error) {
      handleTwitchStatusError(
        state.isTwitchStatusLoaded,
        {
          setError: deps.setError,
          setLoaded: setters.setIsTwitchStatusLoaded,
          setStatus: setters.setTwitchStatus,
        },
        error,
      );
    }
  }, [state.isTwitchStatusLoaded, deps, setters]);

  const loadLiveStatusInternal = useCallback(async (): Promise<void> => {
    try {
      const status = await getLiveStatus();
      handleLiveStatusSuccess(status.channels, {
        setError: setters.setLiveStatusError,
        setLoaded: setters.setIsLiveStatusLoaded,
        setStatus: setters.setLiveStatus,
      });
    } catch {
      handleLiveStatusError({
        setError: setters.setLiveStatusError,
        setLoaded: setters.setIsLiveStatusLoaded,
        setStatus: setters.setLiveStatus,
      });
    }
  }, [setters]);

  const loadChannelsInternal = useCallback(async (): Promise<void> => {
    const twitchPromise = loadTwitchStatus();
    try {
      const newChannels = await getChannels();
      handleChannelsLoadSuccess(newChannels, {
        onChannelsLoaded: deps.onChannelsLoaded,
        setChannels: setters.setChannels,
        setError: deps.setError,
        setLoaded: setters.setIsChannelsLoaded,
      });
      await loadLiveStatusInternal();
    } catch (error) {
      handleChannelsLoadError(
        state.channels,
        {
          onChannelsLoaded: deps.onChannelsLoaded,
          setChannels: setters.setChannels,
          setError: deps.setError,
          setLoaded: setters.setIsChannelsLoaded,
        },
        error,
      );
    }
    await twitchPromise;
  }, [state.channels, deps, setters, loadTwitchStatus, loadLiveStatusInternal]);

  return {
    loadChannelsInternal,
    loadLiveStatusInternal,
    loadTwitchStatus,
  };
};
