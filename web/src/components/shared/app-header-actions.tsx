import { Ellipsis } from 'lucide-react';
import { useCallback, useState, type ReactElement } from 'react';
import type { TwitchStatusResponse } from '../../api-client/types';

interface HeaderActionsProps {
  readonly twitchStatus: TwitchStatusResponse;
  readonly isTwitchStatusLoaded: boolean;
  readonly isTwitchBusy: boolean;
  readonly isBusy: boolean;
  readonly onConnectTwitch: () => void;
  readonly onDisconnectTwitch: () => void;
  readonly onSignOut: () => void;
}

export const HeaderActions = ({
  twitchStatus,
  isTwitchStatusLoaded,
  isTwitchBusy,
  isBusy,
  onConnectTwitch,
  onDisconnectTwitch,
  onSignOut,
}: HeaderActionsProps): ReactElement => {
  const [menuOpen, setMenuOpen] = useState(false);

  const toggleMenu = useCallback((): void => {
    setMenuOpen((prev) => !prev);
  }, []);

  const closeMenu = useCallback((): void => {
    setMenuOpen(false);
  }, []);

  return (
    <>
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

      {menuOpen && (
        <div className="menu-backdrop" onClick={closeMenu} aria-hidden="true" role="presentation" />
      )}
    </>
  );
};
