import { AppBar, Toolbar, Typography, Chip, Box } from '@mui/material';
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
  const getToggleTooltip = useCallback(
    (): string => (relayMode === 'twitch' ? 'Switch to YouTube Relay' : 'Switch to Twitch Relay'),
    [relayMode],
  );

  const getTitle = useCallback(
    (): string => (relayMode === 'twitch' ? 'Twitch Relay' : 'YouTube Relay'),
    [relayMode],
  );

  // Render subtitle for the RelayHeader when authenticated
  const headerSubtitle = (): ReactElement => {
    if (relayMode === 'twitch') {
      if (twitchStatus.connected) {
        return (
          <Chip
            label={`Linked as ${twitchStatus.display_name ?? twitchStatus.login}`}
            color="success"
            size="small"
          />
        );
      }
      return <Chip label="Twitch not connected" color="default" size="small" />;
    }
    return <>Invidious subscriptions</>;
  };

  // Unauthenticated simple header
  if (authMode !== 'authenticated') {
    return (
      <AppBar
        color="default"
        elevation={0}
        position="static"
        sx={{ backgroundColor: 'transparent', padding: 2 }}
      >
        <Toolbar disableGutters sx={{ alignItems: 'flex-start', flexDirection: 'column' }}>
          <Box sx={{ alignItems: 'center', display: 'flex' }}>
            <Typography
              component="p"
              sx={{ color: 'text.secondary', marginRight: 1, textTransform: 'uppercase' }}
              variant="subtitle2"
            >
              Private Deck
            </Typography>
            <Typography variant="h5" component="h1">
              Twitch Relay
            </Typography>
          </Box>
        </Toolbar>
      </AppBar>
    );
  }

  // Authenticated header using RelayHeader component
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
