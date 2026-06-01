import { useCallback, type ReactElement } from 'react';
import type { TwitchStatusResponse } from '../../api-client/types';
import type { AuthMode } from '../../hooks';
import { HeaderActions } from './app-header-actions';
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
            Linked as <strong>{twitchStatus.display_name ?? twitchStatus.login}</strong>
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
    <RelayHeader
      eyebrow="Private Deck"
      title={getTitle()}
      onToggle={onToggleMode}
      toggleLabel={getToggleTooltip()}
      subtitleSnippet={headerSubtitle()}
    >
      <HeaderActions
        twitchStatus={twitchStatus}
        isTwitchStatusLoaded={isTwitchStatusLoaded}
        isTwitchBusy={isTwitchBusy}
        isBusy={isBusy}
        onConnectTwitch={onConnectTwitch}
        onDisconnectTwitch={onDisconnectTwitch}
        onSignOut={onSignOut}
      />
    </RelayHeader>
  );
};
