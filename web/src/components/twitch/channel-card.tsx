import { AlarmClock, Circle, Loader, Play, X } from 'lucide-react';
import type { ReactElement } from 'react';
import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
} from '../../api-client/types';
import { getRecordingTitle, getRecordingLabel, getRecordingClass } from './channel-card-helpers';

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

  const watchButtonClass = isWatching ? 'icon-btn play-btn watching' : 'icon-btn play-btn';
  const watchButtonTitle = isWatching ? 'Opening...' : 'Watch';
  const watchButtonAriaLabel = isWatching ? 'Opening stream...' : 'Watch stream';

  const clockButtonClass =
    recordingRule?.enabled === true ? 'icon-btn clock-btn enabled' : 'icon-btn clock-btn';
  const clockButtonTitle =
    recordingRule?.enabled === true ? 'Disable auto-record' : 'Enable auto-record';
  const clockButtonAriaLabel =
    recordingRule?.enabled === true ? 'Disable auto-record' : 'Enable auto-record';

  const recordingButtonTitle = getRecordingTitle(activeRecording);
  const recordingButtonAriaLabel = getRecordingLabel(activeRecording);
  const recordingButtonClass = `icon-btn record-btn ${getRecordingClass(activeRecording)}`;

  const displayName = getDisplayName(status?.display_name, channel.display_name, channel.login);
  const subtitleText = getSubtitleText(status);

  return (
    <article className={`channel-card ${status?.live === true ? 'live' : ''}`}>
      <div className="channel-avatar-wrap">
        {channel.image_url !== undefined && channel.image_url !== '' ? (
          <img className="ui-avatar channel-avatar" src={channel.image_url} alt={channel.login} />
        ) : (
          <div className="ui-avatar ui-avatar-fallback channel-avatar fallback" aria-hidden="true">
            {channel.login.slice(SLICE_START_INDEX, SLICE_END_INDEX)}
          </div>
        )}
        {status?.live === true && <span className="avatar-status-dot" aria-hidden="true"></span>}
      </div>

      <div className="channel-content">
        <div className="channel-content-header">
          <div className="channel-name-area">
            <button type="button" className="channel-name" onClick={onOpenSetup}>
              {displayName}
            </button>
            <p className="channel-meta">{sourceLabel}</p>
          </div>

          <div className="channel-controls">
            {status?.live === true && (
              <button
                type="button"
                className={watchButtonClass}
                onClick={onStartWatching}
                disabled={isWatching}
                title={watchButtonTitle}
                aria-label={watchButtonAriaLabel}
              >
                {isWatching ? <Loader size={18} className="spinning" /> : <Play size={18} />}
              </button>
            )}
            <button
              type="button"
              className={clockButtonClass}
              title={clockButtonTitle}
              aria-label={clockButtonAriaLabel}
              onClick={onToggleAutoRecord}
            >
              <AlarmClock size={18} />
            </button>
            <button
              type="button"
              className={recordingButtonClass}
              title={recordingButtonTitle}
              aria-label={recordingButtonAriaLabel}
              onClick={onToggleManualRecording}
            >
              <Circle size={16} fill="currentColor" />
            </button>
            {channel.removable && (
              <button
                type="button"
                className="icon-btn remove-btn"
                onClick={onRemove}
                title="Remove channel"
                aria-label="Remove channel"
              >
                <X size={18} />
              </button>
            )}
          </div>
        </div>

        <div className="channel-content-body">
          {status?.live === true && status.title !== undefined && status.title !== '' && (
            <p className="channel-title" title={status.title}>
              {status.title}
            </p>
          )}
          <p className="channel-subtitle">{subtitleText}</p>
        </div>
      </div>
    </article>
  );
};
