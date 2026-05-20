import {
  addChannel,
  createWatchTicket,
  disconnectTwitch,
  getCachedChannels,
  getCachedLiveStatus,
  getChannels,
  getLiveStatus,
  getTwitchConnectUrl,
  getTwitchStatus,
  removeChannel,
} from '$lib/api-client';
import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '$lib/api-client/types';
import { navigate } from '$lib/router/router.svelte';
import {
  type ChannelAddContext,
  type ChannelRemoveContext,
  type ControllerMutableState,
  type TwitchUnlinkContext,
  type WatchContext,
  clearCachedTwitchStatus,
  createInitialTwitchStatus,
  executeAddChannel,
  executeRemoveChannel,
  executeStartWatching,
  executeUnlinkTwitch,
  handleChannelsLoadError,
  handleChannelsLoadSuccess,
  handleLiveStatusError,
  handleLiveStatusSuccess,
  handleTwitchStatusError,
  handleTwitchStatusSuccess,
  loadCachedTwitchStatus,
  resetControllerState,
  validateChannelLogin,
} from './channels-controller.helpers';

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

interface StateSetters {
  setAddingChannel: (value: boolean) => void;
  setChannels: (value: ChannelEntry[]) => void;
  setChannelsLoaded: (value: boolean) => void;
  setLiveStatus: (value: Record<string, ChannelStatus>) => void;
  setLiveStatusError: (value: string | null) => void;
  setLiveStatusLoaded: (value: boolean) => void;
  setRemovingChannel: (value: boolean) => void;
  setTwitchBusy: (value: boolean) => void;
  setTwitchStatus: (value: TwitchStatusResponse) => void;
  setTwitchStatusLoaded: (value: boolean) => void;
  setWatchingChannel: (value: string | null) => void;
}

interface StateRefs {
  channels: ChannelEntry[];
  isTwitchStatusLoaded: boolean;
}

interface ControllerContext {
  onChannelsLoaded: (() => Promise<void>) | undefined;
  refs: StateSetters;
  setError: (msg: string | null) => void;
  state: StateRefs;
}

const loadTwitchStatus = async (
  isLoaded: boolean,
  refs: StateSetters,
  setError: (msg: string | null) => void,
): Promise<void> => {
  try {
    const newStatus = await getTwitchStatus();
    handleTwitchStatusSuccess(newStatus, {
      setError,
      setLoaded: refs.setTwitchStatusLoaded,
      setStatus: refs.setTwitchStatus,
    });
  } catch (error) {
    handleTwitchStatusError(
      isLoaded,
      { setError, setLoaded: refs.setTwitchStatusLoaded, setStatus: refs.setTwitchStatus },
      error,
    );
  }
};

const loadLiveStatus = async (refs: StateSetters): Promise<void> => {
  try {
    const status = await getLiveStatus();
    handleLiveStatusSuccess(status.channels, {
      setError: refs.setLiveStatusError,
      setLoaded: refs.setLiveStatusLoaded,
      setStatus: refs.setLiveStatus,
    });
  } catch {
    handleLiveStatusError({
      setError: refs.setLiveStatusError,
      setLoaded: refs.setLiveStatusLoaded,
      setStatus: refs.setLiveStatus,
    });
  }
};

const loadChannels = async (ctx: ControllerContext): Promise<void> => {
  const twitchPromise = loadTwitchStatus(ctx.state.isTwitchStatusLoaded, ctx.refs, ctx.setError);

  try {
    const newChannels = await getChannels();
    handleChannelsLoadSuccess(newChannels, {
      onChannelsLoaded: ctx.onChannelsLoaded,
      setChannels: ctx.refs.setChannels,
      setError: ctx.setError,
      setLoaded: ctx.refs.setChannelsLoaded,
    });
    await loadLiveStatus(ctx.refs);
  } catch (error) {
    handleChannelsLoadError(
      ctx.state.channels,
      {
        onChannelsLoaded: ctx.onChannelsLoaded,
        setChannels: ctx.refs.setChannels,
        setError: ctx.setError,
        setLoaded: ctx.refs.setChannelsLoaded,
      },
      error,
    );
  }

  await twitchPromise;
};

interface ActionContext {
  addChannelCtx: () => ChannelAddContext;
  removeChannelCtx: () => ChannelRemoveContext;
  unlinkTwitchCtx: () => TwitchUnlinkContext;
  watchCtx: () => WatchContext;
}

const createActionContext = (refs: StateSetters, ctx: ControllerContext): ActionContext => ({
  addChannelCtx: (): ChannelAddContext => ({
    addChannelFn: addChannel,
    loadChannelsFn: async (): Promise<void> => {
      await loadChannels(ctx);
    },
    setAdding: refs.setAddingChannel,
    setError: ctx.setError,
  }),
  removeChannelCtx: (): ChannelRemoveContext => ({
    loadChannelsFn: async (): Promise<void> => {
      await loadChannels(ctx);
    },
    removeChannelFn: removeChannel,
    setError: ctx.setError,
    setRemoving: refs.setRemovingChannel,
  }),
  unlinkTwitchCtx: (): TwitchUnlinkContext => ({
    disconnectFn: disconnectTwitch,
    loadChannelsFn: async (): Promise<void> => {
      await loadChannels(ctx);
    },
    setBusy: refs.setTwitchBusy,
    setError: ctx.setError,
    setStatus: refs.setTwitchStatus,
  }),
  watchCtx: (): WatchContext => ({
    navigateFn: navigate,
    setError: ctx.setError,
    setWatching: refs.setWatchingChannel,
  }),
});

interface Actions {
  confirmRemoveChannel: (login: string) => Promise<void>;
  connectTwitch: () => void;
  loadChannels: () => Promise<void>;
  loadLiveStatus: () => Promise<void>;
  startWatching: (channelLogin: string) => Promise<void>;
  submitAddChannel: (newChannelLogin: string) => Promise<void>;
  unlinkTwitch: () => Promise<void>;
}

const createActions = (refs: StateSetters, ctx: ControllerContext): Actions => {
  const actionCtx = createActionContext(refs, ctx);

  const submitAddChannel = async (newChannelLogin: string): Promise<void> => {
    const normalized = validateChannelLogin(newChannelLogin);
    if (normalized === null) {
      ctx.setError('channel name is required');
      return;
    }
    await executeAddChannel(normalized, actionCtx.addChannelCtx());
  };

  const confirmRemoveChannel = async (login: string): Promise<void> => {
    await executeRemoveChannel(login, actionCtx.removeChannelCtx());
  };

  const connectTwitch = (): void => {
    globalThis.window.location.assign(getTwitchConnectUrl());
  };

  const unlinkTwitch = async (): Promise<void> => {
    await executeUnlinkTwitch(actionCtx.unlinkTwitchCtx());
  };

  const startWatching = async (channelLogin: string): Promise<void> => {
    await executeStartWatching(channelLogin, createWatchTicket, actionCtx.watchCtx());
  };

  return {
    confirmRemoveChannel,
    connectTwitch,
    loadChannels: async (): Promise<void> => {
      await loadChannels(ctx);
    },
    loadLiveStatus: async (): Promise<void> => {
      await loadLiveStatus(refs);
    },
    startWatching,
    submitAddChannel,
    unlinkTwitch,
  };
};

export const createChannelsController = (
  deps: Readonly<ChannelsControllerDeps>,
): ChannelsController => {
  const cachedStatus = loadCachedTwitchStatus();
  const cachedChannels = getCachedChannels();
  const cachedLiveStatus = getCachedLiveStatus();
  const initialStatus = createInitialTwitchStatus(cachedStatus);

  let channels = $state<ChannelEntry[]>(cachedChannels);
  let isChannelsLoaded = $state<boolean>(cachedChannels.length > EMPTY_ARRAY_LENGTH);
  let liveStatus = $state<Record<string, ChannelStatus>>(cachedLiveStatus);
  let isLiveStatusLoaded = $state<boolean>(
    Object.keys(cachedLiveStatus).length > EMPTY_OBJECT_KEYS_LENGTH,
  );
  let liveStatusError = $state<string | null>(null);
  let twitchStatus = $state<TwitchStatusResponse>(initialStatus);
  let isTwitchStatusLoaded = $state<boolean>(cachedStatus !== null);
  let isTwitchBusy = $state(false);
  let watchingChannel = $state<string | null>(null);
  let isAddingChannel = $state(false);
  let isRemovingChannel = $state(false);

  const { onChannelsLoaded, setError } = deps;

  const refs: StateSetters = {
    setAddingChannel: (value) => {
      isAddingChannel = value;
    },
    setChannels: (value) => {
      channels = value;
    },
    setChannelsLoaded: (value) => {
      isChannelsLoaded = value;
    },
    setLiveStatus: (value) => {
      liveStatus = value;
    },
    setLiveStatusError: (value) => {
      liveStatusError = value;
    },
    setLiveStatusLoaded: (value) => {
      isLiveStatusLoaded = value;
    },
    setRemovingChannel: (value) => {
      isRemovingChannel = value;
    },
    setTwitchBusy: (value) => {
      isTwitchBusy = value;
    },
    setTwitchStatus: (value) => {
      twitchStatus = value;
    },
    setTwitchStatusLoaded: (value) => {
      isTwitchStatusLoaded = value;
    },
    setWatchingChannel: (value) => {
      watchingChannel = value;
    },
  };

  const mutableCtx: ControllerMutableState = {
    setAddingChannel: refs.setAddingChannel,
    setChannels: refs.setChannels,
    setChannelsLoaded: refs.setChannelsLoaded,
    setLiveStatus: refs.setLiveStatus,
    setLiveStatusError: refs.setLiveStatusError,
    setLiveStatusLoaded: refs.setLiveStatusLoaded,
    setRemovingChannel: refs.setRemovingChannel,
    setTwitchBusy: refs.setTwitchBusy,
    setTwitchStatus: refs.setTwitchStatus,
    setTwitchStatusLoaded: refs.setTwitchStatusLoaded,
    setWatchingChannel: refs.setWatchingChannel,
  };

  const ctx: ControllerContext = {
    onChannelsLoaded,
    refs,
    setError,
    state: { channels, isTwitchStatusLoaded },
  };

  const actions = createActions(refs, ctx);

  const resetState = (): void => {
    resetControllerState(mutableCtx);
    clearCachedTwitchStatus();
  };

  return {
    get channels() {
      return channels;
    },
    confirmRemoveChannel: actions.confirmRemoveChannel,
    connectTwitch: actions.connectTwitch,
    get isAddingChannel() {
      return isAddingChannel;
    },
    get isChannelsLoaded() {
      return isChannelsLoaded;
    },
    get isLiveStatusLoaded() {
      return isLiveStatusLoaded;
    },
    get isRemovingChannel() {
      return isRemovingChannel;
    },
    get isTwitchBusy() {
      return isTwitchBusy;
    },
    get isTwitchStatusLoaded() {
      return isTwitchStatusLoaded;
    },
    get liveStatus() {
      return liveStatus;
    },
    get liveStatusError() {
      return liveStatusError;
    },
    loadChannels: actions.loadChannels,
    loadLiveStatus: actions.loadLiveStatus,
    resetState,
    startWatching: actions.startWatching,
    submitAddChannel: actions.submitAddChannel,
    get twitchStatus() {
      return twitchStatus;
    },
    unlinkTwitch: actions.unlinkTwitch,
    get watchingChannel() {
      return watchingChannel;
    },
  };
};
