import { AlarmClock, Circle, Loader, Play, X } from 'lucide-react';
import type { ReactElement } from 'react';
import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
} from '../../api-client/types';

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
  const SLICE_START_INDEX = 0;
const SLICE_END_INDEX = 1;

const sourceLabel = ((): string => {
    if (channel.source === 'manual') {
      return 'Manual';
    }
    if (channel.source === 'followed') {
      return 'Followed';
    }
    return 'Manual + Followed';
  })();

  const watchButtonClass = isWatching ? 'icon-btn play-btn watching' : 'icon-btn play-btn';
  const watchButtonTitle = isWatching ? 'Opening...' : 'Watch';
  const watchButtonAriaLabel = isWatching ? 'Opening stream...' : 'Watch stream';

  const clockButtonClass = recordingRule?.enabled === true
    ? 'icon-btn clock-btn enabled'
    : 'icon-btn clock-btn';
  const clockButtonTitle = recordingRule?.enabled === true ? 'Disable auto-record' : 'Enable auto-record';
  const clockButtonAriaLabel = recordingRule?.enabled === true
    ? 'Disable auto-record'
    : 'Enable auto-record';

  const getRecordingTitle = (): string => {
    if (activeRecording?.mode === 'manual') {
      return 'Stop manual recording';
    }
    if (activeRecording?.mode === 'auto') {
      return 'Stop auto recording';
    }
    return 'Start recording now';
  };

  const getRecordingLabel = (): string => {
    if (activeRecording?.mode === 'manual') {
      return 'Stop manual recording';
    }
    if (activeRecording?.mode === 'auto') {
      return 'Stop auto recording';
    }
    return 'Start recording now';
  };

  const getRecordingClass = (): string => {
    if (activeRecording?.mode === 'manual') {
      return 'active-manual';
    }
    if (activeRecording?.mode === 'auto') {
      return 'active-auto';
    }
    return '';
  };

  const recordingButtonTitle = getRecordingTitle();
  const recordingButtonAriaLabel = getRecordingLabel();
  const recordingButtonClass = `icon-btn record-btn ${getRecordingClass()}`;

  const subtitleText = ((): string => {
    if (status?.live === true && status.game !== undefined && status.game !== '') {
      return `Playing: ${status.game}`;
    }
    const viewerCount = status?.viewer_count;
    if (status?.live === true && typeof viewerCount === 'number' && viewerCount > 0) {
      return `${viewerCount.toLocaleString()} viewers`;
    }
    return 'Offline';
  })();

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
              {(status?.display_name !== undefined && status.display_name !== '')
                ? status.display_name
                : (channel.display_name !== undefined && channel.display_name !== '')
                  ? channel.display_name
                  : channel.login}
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
}
