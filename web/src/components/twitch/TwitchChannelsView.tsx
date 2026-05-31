import { useCallback, useMemo, useState, type ReactElement } from 'react';
import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
} from '../../api-client/types';
import { EmptyState } from '../ui/EmptyState';
import { AddChannelForm } from './AddChannelForm';
import { ChannelCard } from './ChannelCard';

const LIVE_ONLY_PREF_KEY = 'twitchRelay.liveOnly';
const LIVE_ONLY_ENABLED = '1';
const LIVE_ONLY_DISABLED = '0';

const loadLiveOnlyPreference = (): boolean => {
  try {
    return localStorage.getItem(LIVE_ONLY_PREF_KEY) === LIVE_ONLY_ENABLED;
  } catch {
    return false;
  }
};

const saveLiveOnlyPreference = (value: boolean): void => {
  try {
    localStorage.setItem(LIVE_ONLY_PREF_KEY, value ? LIVE_ONLY_ENABLED : LIVE_ONLY_DISABLED);
  } catch {
    // Ignore storage failures and keep in-memory state
  }
};

interface TwitchChannelsViewProps {
  channels: ChannelEntry[];
  liveStatus: Record<string, ChannelStatus>;
  showAddForm: boolean;
  newChannelLogin: string;
  isAddingChannel: boolean;
  watchingChannel: string | undefined;
  recordingRules: Record<string, RecordingRule | undefined>;
  activeRecordings: Record<string, ActiveRecording | undefined>;
  liveStatusError: string | undefined;
  isLiveStatusLoaded: boolean;
  onOpenRecordings: () => void;
  onShowAddForm: () => void;
  onCancelAddForm: () => void;
  onSubmitAddChannel: (event: React.FormEvent) => void;
  onUpdateNewChannelLogin: (value: string) => void;
  onOpenChannelSetup: (login: string) => void;
  onStartWatching: (login: string) => void;
  onToggleAutoRecord: (login: string) => void;
  onToggleManualRecording: (login: string) => void;
  onPromptRemoveChannel: (login: string) => void;
}

export const TwitchChannelsView = ({
  channels,
  liveStatus,
  showAddForm,
  newChannelLogin,
  isAddingChannel,
  watchingChannel,
  recordingRules,
  activeRecordings,
  liveStatusError,
  isLiveStatusLoaded,
  onOpenRecordings,
  onShowAddForm,
  onCancelAddForm,
  onSubmitAddChannel,
  onUpdateNewChannelLogin,
  onOpenChannelSetup,
  onStartWatching,
  onToggleAutoRecord,
  onToggleManualRecording,
  onPromptRemoveChannel,
}: TwitchChannelsViewProps): ReactElement => {
  const [liveOnly, setLiveOnly] = useState(loadLiveOnlyPreference);

  const handleLiveOnlyChange = useCallback((value: boolean) => {
    setLiveOnly(value);
    saveLiveOnlyPreference(value);
  }, []);

  const visibleChannels = useMemo(() => {
    if (!liveOnly) {
      return channels;
    }
    // If liveOnly is enabled but we haven't loaded live status yet, show all cached channels
    // This prevents false "No channels are live" message
    if (!isLiveStatusLoaded) {
      return channels;
    }
    return channels.filter((channel) => Boolean(liveStatus[channel.login]?.live));
  }, [channels, liveOnly, liveStatus, isLiveStatusLoaded]);

  return (
    <div>
      <div className="channels-header">
        <div className="channels-title-row">
          <span className="channels-label">Channels</span>
          <label className="live-only-switch" aria-label="Show only live channels">
            <span className="switch-text">Live only</span>
            <input
              className="switch-input"
              type="checkbox"
              checked={liveOnly}
              onChange={(event) => handleLiveOnlyChange(event.currentTarget.checked)}
            />
            <span className="switch-track" aria-hidden="true">
              <span className="switch-knob"></span>
            </span>
          </label>
        </div>
        <div className="channels-actions">
          <button type="button" className="ui-nav-chip" onClick={onOpenRecordings}>
            Recordings overview
          </button>
          {!showAddForm && (
            <button type="button" className="add-btn" onClick={onShowAddForm}>
              + Add channel
            </button>
          )}
        </div>
      </div>

      {liveStatusError && <p className="live-status-warning">{liveStatusError}</p>}

      {showAddForm && (
        <AddChannelForm
          newChannelLogin={newChannelLogin}
          isAdding={isAddingChannel}
          onSubmit={onSubmitAddChannel}
          onCancel={onCancelAddForm}
          onUpdateValue={onUpdateNewChannelLogin}
        />
      )}

      <div className="channels">
        {visibleChannels.length === 0 ? (
          liveOnly && isLiveStatusLoaded ? (
            <EmptyState
              title="No channels are live"
              description="Toggle off 'Live only' to see all configured channels."
              variant="channels"
            />
          ) : (
            <EmptyState
              title="No channels configured"
              description="Add Twitch channels to see their live status here."
              variant="channels"
            />
          )
        ) : (
          visibleChannels.map((channel) => (
            <ChannelCard
              key={channel.login}
              channel={channel}
              status={liveStatus[channel.login]}
              recordingRule={recordingRules[channel.login]}
              activeRecording={activeRecordings[channel.login]}
              isWatching={watchingChannel === channel.login}
              onOpenSetup={() => onOpenChannelSetup(channel.login)}
              onStartWatching={() => onStartWatching(channel.login)}
              onToggleAutoRecord={() => onToggleAutoRecord(channel.login)}
              onToggleManualRecording={() => onToggleManualRecording(channel.login)}
              onRemove={() => onPromptRemoveChannel(channel.login)}
            />
          ))
        )}
      </div>
    </div>
  );
}
