import { Maximize2, X } from 'lucide-react';
import { useEffect, useState, type ReactElement } from 'react';
import { WatchPage } from '../../pages/watch-page';
import { navigate } from '../../router';

interface PersistentWatchPlayerProps {
  path: string;
  routeTicket: string;
}

const WATCH_PREFIX = '/watch/';
const TWITCH_HOME = '/twitch';

export const PersistentWatchPlayer = ({
  path,
  routeTicket,
}: PersistentWatchPlayerProps): ReactElement | null => {
  const isWatchRoute = path.startsWith(WATCH_PREFIX);
  const [activeTicket, setActiveTicket] = useState(() => (isWatchRoute ? routeTicket : ''));

  useEffect(() => {
    if (isWatchRoute && routeTicket !== '') {
      setActiveTicket(routeTicket);
    }
  }, [isWatchRoute, routeTicket]);

  if (activeTicket === '') {
    return null;
  }

  const isMinimized = path === TWITCH_HOME && !isWatchRoute;
  if (!isWatchRoute && !isMinimized) {
    return null;
  }

  const restorePlayer = (): void => {
    navigate(`${WATCH_PREFIX}${encodeURIComponent(activeTicket)}`);
  };

  const closePlayer = (): void => {
    setActiveTicket('');
  };

  return (
    <div
      className={
        isMinimized ? 'persistent-watch persistent-watch--minimized' : 'persistent-watch'
      }
    >
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
    </div>
  );
};
