import { Users } from 'lucide-react';
import type { ReactElement } from 'react';

const FALLBACK_INITIAL_LENGTH = 1;
const FALLBACK_INITIAL_START = 0;
const VIEWER_COUNT_LOCALE = 'en-US';

interface WatchPageMetaProps {
  channelLogin: string;
  displayName?: string;
  game?: string;
  live: boolean;
  profileUrl?: string;
  title?: string;
  viewerCount?: number;
}

const formatViewerCount = (count: number): string =>
  new Intl.NumberFormat(VIEWER_COUNT_LOCALE).format(count);

export const WatchPageMeta = ({
  channelLogin,
  displayName,
  game,
  live,
  profileUrl,
  title,
  viewerCount,
}: WatchPageMetaProps): ReactElement => {
  const trimmedDisplayName = displayName?.trim();
  const shownName = trimmedDisplayName !== undefined && trimmedDisplayName !== ''
    ? trimmedDisplayName
    : channelLogin;
  const showLogin = trimmedDisplayName !== undefined && trimmedDisplayName !== ''
    && trimmedDisplayName.toLowerCase() !== channelLogin.toLowerCase();

  return (
    <div className="watch-page-meta">
      {profileUrl !== undefined && profileUrl !== '' ? (
        <img alt={`${shownName} avatar`} className="ui-avatar watch-page-avatar" src={profileUrl} />
      ) : (
        <div className="ui-avatar ui-avatar-fallback watch-page-avatar">
          {shownName.slice(FALLBACK_INITIAL_START, FALLBACK_INITIAL_LENGTH)}
        </div>
      )}

      <div className="watch-page-meta-text">
        <div className="watch-page-name-row">
          <span className="watch-page-name">{shownName}</span>
          {showLogin && <span className="watch-page-login">{channelLogin}</span>}
          <span className={`watch-page-live-badge ${live ? 'live' : 'offline'}`}>
            <span className="watch-page-live-dot" />
            {live ? 'LIVE' : 'OFFLINE'}
          </span>
          {live && typeof viewerCount === 'number' && (
            <span className="watch-page-viewers">
              <Users size={14} />
              {formatViewerCount(viewerCount)}
            </span>
          )}
        </div>
        <div className="watch-page-detail-row">
          {title !== undefined && title !== '' && (
            <span className="watch-page-title-text">{title}</span>
          )}
          {game !== undefined && game !== '' && (
            <span className="watch-page-game">{game}</span>
          )}
        </div>
      </div>
    </div>
  );
};
