import { Ellipsis } from 'lucide-react';
import { useCallback, useState, type ReactElement } from 'react';
import type { TwitchStatusResponse } from '../../api-client/types';
import type { AuthMode } from '../../hooks';
import { RelayHeader } from './relay-header';

interface AppHeaderProps {
  authMode: AuthMode;
  relayMode: 'twitch' | 'youtube';
  twitchStatus: TwitchStatusResponse;
  isTwitchStatusLoaded: boolean;
  isTwitchBusy: boolean;
  isBusy: boolean;
  onToggleMode: () => void;
  onConnectTwitch: () => void;
  onDisconnectTwitch: () => void;
  onSignOut: () => void;
}

export const AppHeader = ({
  authMode,
  relayMode,
  twitchStatus,
  isTwitchStatusLoaded,
  isTwitchBusy,
  isBusy,
  onToggleMode,
  onConnectTwitch,
  onDisconnectTwitch,
  onSignOut,
}: AppHeaderProps): ReactElement => {
  const [menuOpen, setMenuOpen] = useState(false);

  const getToggleTooltip = useCallback((): string => {
    if (relayMode === 'twitch') {
      return 'Switch to YouTube Relay';
    }
    return 'Switch to Twitch Relay';
  }, [relayMode]);

  const getTitle = useCallback((): string => {
    if (relayMode === 'twitch') {
      return 'Twitch Relay';
    }
    return 'YouTube Relay';
  }, [relayMode]);

  const toggleMenu = useCallback((): void => {
    setMenuOpen((prev) => !prev);
  }, []);

  const closeMenu = useCallback((): void => {
    setMenuOpen(false);
  }, []);

  if (authMode !== 'authenticated') {
    return (
      <header className="app-header-simple">
        <div className="app-header-title">
          <p className="app-header-eyebrow">Private Deck</p>
          <h1>Twitch Relay</h1>
        </div>
      </header>
    );
  }

  const headerSubtitle = (): ReactElement => {
    if (relayMode === 'twitch') {
      if (twitchStatus.connected) {
        return (
          <>
            <span className="status-dot connected" aria-hidden="true" />
            Linked as           <strong>{twitchStatus.display_name ?? twitchStatus.login}</strong>
          </>
        );
      }
      return (
        <>
          <span className="status-dot disconnected" aria-hidden="true" />
          Twitch not connected
        </>
      );
    }
    return <>Invidious subscriptions</>;
  };

  return (
    <>
      <RelayHeader
        eyebrow="Private Deck"
        title={getTitle()}
        onToggle={onToggleMode}
        toggleLabel={getToggleTooltip()}
        subtitleSnippet={headerSubtitle()}
      >
        <div className="header-actions">
          {/* Desktop: inline buttons */}
          <div className="header-actions-inline">
            {isTwitchStatusLoaded ? (
              twitchStatus.connected ? (
                <button
                  type="button"
                  className="ui-nav-chip"
                  onClick={onDisconnectTwitch}
                  disabled={isTwitchBusy}
                >
                  {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
                </button>
              ) : (
                <button type="button" className="ui-nav-chip" onClick={onConnectTwitch}>
                  Connect Twitch
                </button>
              )
            ) : (
              <button type="button" className="ui-nav-chip" disabled aria-busy="true">
                Loading...
              </button>
            )}
            <button type="button" className="ui-nav-chip" onClick={onSignOut} disabled={isBusy}>
              Sign out
            </button>
          </div>

          {/* Mid-size: collapsed menu button */}
          <div className="header-actions-menu">
            <button
              type="button"
              className="ui-nav-chip menu-toggle"
              onClick={toggleMenu}
              aria-label="Menu"
              aria-expanded={menuOpen}
            >
              <Ellipsis size={18} />
            </button>

            {menuOpen && (
              <div className="menu-dropdown" role="menu">
                {isTwitchStatusLoaded ? (
                  twitchStatus.connected ? (
                    <button
                      type="button"
                      className="menu-item"
                      onClick={() => {
                        closeMenu();
                        onDisconnectTwitch();
                      }}
                      disabled={isTwitchBusy}
                    >
                      {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="menu-item"
                      onClick={() => {
                        closeMenu();
                        onConnectTwitch();
                      }}
                    >
                      Connect Twitch
                    </button>
                  )
                ) : (
                  <button type="button" className="menu-item" disabled aria-busy="true">
                    Loading...
                  </button>
                )}
                <button
                  type="button"
                  className="menu-item"
                  onClick={() => {
                    closeMenu();
                    onSignOut();
                  }}
                  disabled={isBusy}
                >
                  Sign out
                </button>
              </div>
            )}
          </div>
        </div>
      </RelayHeader>

      {menuOpen && (
        <div className="menu-backdrop" onClick={closeMenu} aria-hidden="true" role="presentation" />
      )}
    </>
  );
}
