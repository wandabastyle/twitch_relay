import type { ChannelEntry, ChannelStatus, TwitchStatusResponse } from '../api-client/types.js';

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

export { useChannelsController } from './useChannelsController.js';
