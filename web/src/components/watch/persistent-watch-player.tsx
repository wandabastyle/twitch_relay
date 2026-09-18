import { Maximize2, X } from 'lucide-react';
import { useCallback, useEffect, useState, type ReactElement } from 'react';
import { useRaidFollow } from '../../hooks/watch/use-raid-follow';
import { WatchPage } from '../../pages/watch-page';
import { navigate } from '../../router';

interface PersistentWatchPlayerProps {
  path: string;
  routeTicket: string;
}

const WATCH_PREFIX = '/watch/';
const TWITCH_HOME = '/twitch';
const TICKET_MATCH_INDEX = 1;

export const PersistentWatchPlayer = ({
  path,
  routeTicket,
}: PersistentWatchPlayerProps): ReactElement | null => {
  const isWatchRoute = path.startsWith(WATCH_PREFIX);
  const [activeTicket, setActiveTicket] = useState(() => (isWatchRoute ? routeTicket : ''));
  const [raidError, setRaidError] = useState('');
  const isMinimized = path === TWITCH_HOME && !isWatchRoute;
  const isPlayerActive = isWatchRoute || isMinimized;

  useEffect(() => {
    if (isWatchRoute && routeTicket !== '') {
      setActiveTicket(routeTicket);
    }
  }, [isWatchRoute, routeTicket]);

  useEffect(() => {
    if (!isPlayerActive) {
      setActiveTicket('');
      setRaidError('');
    }
  }, [isPlayerActive]);

  const followRaid = useCallback(
    (watchUrl: string): void => {
      const match = /^\/watch\/([^/?#]+)$/.exec(watchUrl);
      if (match === null) {
        setRaidError('Unable to follow raid: watch ticket response is invalid.');
        return;
      }

      const nextTicket = decodeURIComponent(match[TICKET_MATCH_INDEX]);
      setRaidError('');
      setActiveTicket(nextTicket);
      if (isWatchRoute) {
        navigate(watchUrl, { replace: true });
      }
    },
    [isWatchRoute],
  );

  const handleRaidError = useCallback((message: string): void => {
    setRaidError(`Unable to follow raid: ${message}`);
  }, []);

  useRaidFollow({
    enabled: isPlayerActive && activeTicket !== '',
    onError: handleRaidError,
    onFollow: followRaid,
    ticket: activeTicket,
  });

  if (activeTicket === '') {
    return null;
  }

  if (!isWatchRoute && !isMinimized) {
    return null;
  }

  const restorePlayer = (): void => {
    navigate(`${WATCH_PREFIX}${encodeURIComponent(activeTicket)}`);
  };

  const closePlayer = (): void => {
    setActiveTicket('');
    setRaidError('');
  };

  const playerClassName = isMinimized
    ? 'persistent-watch persistent-watch--minimized'
    : 'persistent-watch';

  return (
    <div className={playerClassName}>
      {isMinimized && (
        <div className="persistent-watch-actions">
          <button
            type="button"
            className="persistent-watch-action"
            onClick={restorePlayer}
            aria-label="Return to stream"
            title="Return to stream"
          >
            <Maximize2 aria-hidden="true" size={16} />
          </button>
          <button
            type="button"
            className="persistent-watch-action"
            onClick={closePlayer}
            aria-label="Close stream"
            title="Close stream"
          >
            <X aria-hidden="true" size={16} />
          </button>
        </div>
      )}
      <WatchPage key={activeTicket} ticketOverride={activeTicket} minimized={isMinimized} />
      {raidError !== '' && (
        <p className="ui-error" role="alert">
          {raidError}
        </p>
      )}
    </div>
  );
};
