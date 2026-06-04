import { Button, FormControlLabel, Stack, Switch, Typography } from '@mui/material';
import { useCallback, useMemo, useState, type ReactElement } from 'react';
import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
} from '../../api-client/types';
import { EmptyState } from '../ui/empty-state';
import { AddChannelForm } from './add-channel-form';
import { ChannelCard } from './channel-card';

const EMPTY_LENGTH = 0;
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
  onSubmitAddChannel: (event: React.SyntheticEvent<HTMLFormElement>) => void;
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
    return channels.filter((channel) => liveStatus[channel.login]?.live);
  }, [channels, liveOnly, liveStatus, isLiveStatusLoaded]);

  return (
    <Stack spacing={3}>
      {/* Header */}
      <Stack
        direction="row"
        spacing={2}
        sx={{ alignItems: 'center', justifyContent: 'space-between' }}
      >
        <Typography className="channels-label" component="span" variant="h6">
          Channels
        </Typography>
        <FormControlLabel
          className="live-only-switch"
          control={
            <Switch
              checked={liveOnly}
              color="primary"
              onChange={(event) => {
                handleLiveOnlyChange(event.currentTarget.checked);
              }}
            />
          }
          label="Live only"
        />
      </Stack>

      {/* Actions */}
      <Stack direction="row" spacing={2}>
        <Button className="ui-nav-chip" onClick={onOpenRecordings} variant="outlined">
          Recordings overview
        </Button>
        {!showAddForm && (
          <Button className="add-btn" onClick={onShowAddForm} variant="contained">
            + Add channel
          </Button>
        )}
      </Stack>

      {liveStatusError !== undefined && liveStatusError !== '' && (
        <Typography className="live-status-warning" color="error">
          {liveStatusError}
        </Typography>
      )}

      {showAddForm && (
        <AddChannelForm
          isAdding={isAddingChannel}
          newChannelLogin={newChannelLogin}
          onCancel={onCancelAddForm}
          onSubmit={onSubmitAddChannel}
          onUpdateValue={onUpdateNewChannelLogin}
        />
      )}

      <Stack className="channels" spacing={2}>
        {visibleChannels.length === EMPTY_LENGTH ? (
          liveOnly && isLiveStatusLoaded ? (
            <EmptyState
              description="Toggle off 'Live only' to see all configured channels."
              title="No channels are live"
              variant="channels"
            />
          ) : (
            <EmptyState
              description="Add Twitch channels to see their live status here."
              title="No channels configured"
              variant="channels"
            />
          )
        ) : (
          visibleChannels.map((channel) => (
            <ChannelCard
              activeRecording={activeRecordings[channel.login]}
              channel={channel}
              isWatching={watchingChannel === channel.login}
              key={channel.login}
              recordingRule={recordingRules[channel.login]}
              status={liveStatus[channel.login]}
              onOpenSetup={() => {
                onOpenChannelSetup(channel.login);
              }}
              onRemove={() => {
                onPromptRemoveChannel(channel.login);
              }}
              onStartWatching={() => {
                onStartWatching(channel.login);
              }}
              onToggleAutoRecord={() => {
                onToggleAutoRecord(channel.login);
              }}
              onToggleManualRecording={() => {
                onToggleManualRecording(channel.login);
              }}
            />
          ))
        )}
      </Stack>
    </Stack>
  );
};
