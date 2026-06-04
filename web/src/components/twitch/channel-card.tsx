import AccessTimeIcon from '@mui/icons-material/AccessTime';
import CloseIcon from '@mui/icons-material/Close';
import LensIcon from '@mui/icons-material/Lens';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import {
  CircularProgress,
  Avatar,
  IconButton,
  Card,
  CardContent,
  Box,
  Typography,
  Chip,
} from '@mui/material';
import type { ReactElement } from 'react';
import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
} from '../../api-client/types';
import { getRecordingTitle, getRecordingLabel } from './channel-card-helpers';

interface ChannelCardProps {
  channel: ChannelEntry;
  status: ChannelStatus | undefined;
  recordingRule: RecordingRule | undefined;
  activeRecording: ActiveRecording | undefined;
  isWatching: boolean;
  onOpenSetup: () => void;
  onStartWatching: () => void;
  onToggleAutoRecord: () => void;
  onToggleManualRecording: () => void;
  onRemove: () => void;
}

const MIN_VIEWER_COUNT = 0;
const SLICE_START_INDEX = 0;
const SLICE_END_INDEX = 1;

const getSourceLabel = (source: string): string => {
  if (source === 'manual') {
    return 'Manual';
  }
  if (source === 'followed') {
    return 'Followed';
  }
  return 'Manual + Followed';
};

const getDisplayName = (
  statusDisplayName: string | undefined,
  channelDisplayName: string | undefined,
  login: string,
): string => {
  if (statusDisplayName !== undefined && statusDisplayName !== '') {
    return statusDisplayName;
  }
  if (channelDisplayName !== undefined && channelDisplayName !== '') {
    return channelDisplayName;
  }
  return login;
};

const getSubtitleText = (status: ChannelStatus | undefined): string => {
  if (status?.live === true && status.game !== undefined && status.game !== '') {
    return `Playing: ${status.game}`;
  }
  const viewerCount = status?.viewer_count;
  if (status?.live === true && typeof viewerCount === 'number' && viewerCount > MIN_VIEWER_COUNT) {
    return `${viewerCount.toLocaleString()} viewers`;
  }
  return 'Offline';
};

export const ChannelCard = ({
  channel,
  status,
  recordingRule,
  activeRecording,
  isWatching,
  onOpenSetup,
  onStartWatching,
  onToggleAutoRecord,
  onToggleManualRecording,
  onRemove,
}: ChannelCardProps): ReactElement => {
  const sourceLabel = getSourceLabel(channel.source);

  const watchButtonTitle = isWatching ? 'Opening...' : 'Watch';
  const watchButtonAriaLabel = isWatching ? 'Opening stream...' : 'Watch stream';

  const clockButtonTitle =
    recordingRule?.enabled === true ? 'Disable auto-record' : 'Enable auto-record';
  const clockButtonAriaLabel =
    recordingRule?.enabled === true ? 'Disable auto-record' : 'Enable auto-record';

  const recordingButtonTitle = getRecordingTitle(activeRecording);
  const recordingButtonAriaLabel = getRecordingLabel(activeRecording);

  const displayName = getDisplayName(status?.display_name, channel.display_name, channel.login);
  const subtitleText = getSubtitleText(status);

  return (
    <Card sx={{ marginBottom: 2 }}>
      <CardContent sx={{ alignItems: 'flex-start', display: 'flex', gap: 2 }}>
        <Box sx={{ position: 'relative' }}>
          {channel.image_url !== undefined && channel.image_url !== '' ? (
            <Avatar alt={channel.login} src={channel.image_url} />
          ) : (
            <Avatar>{channel.login.slice(SLICE_START_INDEX, SLICE_END_INDEX)}</Avatar>
          )}
          {status?.live === true && (
            <Chip
              color="error"
              label="Live"
              size="small"
              sx={{ bottom: 0, position: 'absolute', right: 0 }}
            />
          )}
        </Box>

        <Box sx={{ flex: 1 }}>
          <Box sx={{ alignItems: 'center', display: 'flex', justifyContent: 'space-between' }}>
            <Box>
              <Typography
                component="button"
                onClick={onOpenSetup}
                sx={{ background: 'none', border: 'none', cursor: 'pointer', padding: 0 }}
                variant="subtitle1"
              >
                {displayName}
              </Typography>
              <Typography color="text.secondary" variant="caption">
                {sourceLabel}
              </Typography>
            </Box>

            <Box>
              {status?.live === true && (
                <IconButton
                  aria-label={watchButtonAriaLabel}
                  disabled={isWatching}
                  onClick={onStartWatching}
                  title={watchButtonTitle}
                >
                  {isWatching ? <CircularProgress size={18} /> : <PlayArrowIcon fontSize="small" />}
                </IconButton>
              )}
              <IconButton
                aria-label={clockButtonAriaLabel}
                onClick={onToggleAutoRecord}
                sx={{ color: recordingRule?.enabled === true ? 'primary.main' : 'inherit' }}
                title={clockButtonTitle}
              >
                <AccessTimeIcon fontSize="small" />
              </IconButton>
              <IconButton
                aria-label={recordingButtonAriaLabel}
                onClick={onToggleManualRecording}
                sx={{ color: activeRecording === undefined ? 'inherit' : 'primary.main' }}
                title={recordingButtonTitle}
              >
                <LensIcon fontSize="small" />
              </IconButton>
              {channel.removable && (
                <IconButton aria-label="Remove channel" onClick={onRemove} title="Remove channel">
                  <CloseIcon fontSize="small" />
                </IconButton>
              )}
            </Box>
          </Box>

          <Box sx={{ marginTop: 1 }}>
            {status?.live === true && status.title !== undefined && status.title !== '' && (
              <Typography title={status.title} variant="body2">
                {status.title}
              </Typography>
            )}
            <Typography color="text.secondary" variant="caption">
              {subtitleText}
            </Typography>
          </Box>
        </Box>
      </CardContent>
    </Card>
  );
};
