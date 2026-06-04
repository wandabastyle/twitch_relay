import { Menu, MenuItem, IconButton, Button, Box } from '@mui/material';
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
  const [anchorElement, setAnchorElement] = useState<null | HTMLElement>(null);

  const handleMenuOpen = useCallback((event: React.MouseEvent<HTMLElement>) => {
    setAnchorElement(event.currentTarget);
  }, []);

  const handleMenuClose = useCallback(() => {
    setAnchorElement(null);
  }, []);

  const isMenuOpen = Boolean(anchorElement);

  const getTwitchMenuText = (): string => {
    if (!isTwitchStatusLoaded) {
      return 'Loading...';
    }
    if (twitchStatus.connected) {
      return isTwitchBusy ? 'Disconnecting...' : 'Disconnect';
    }
    return 'Connect Twitch';
  };

  const handleTwitchMenuClick = (): void => {
    handleMenuClose();
    if (isTwitchStatusLoaded && twitchStatus.connected) {
      onDisconnectTwitch();
    } else if (isTwitchStatusLoaded) {
      onConnectTwitch();
    }
  };

  const handleSignOutClick = (): void => {
    onSignOut();
    handleMenuClose();
  };

  return (
    <Box sx={{ alignItems: 'center', display: 'flex' }}>
      {/* Desktop: inline buttons */}
      <Box sx={{ alignItems: 'center', display: { md: 'flex', xs: 'none' } }}>
        {isTwitchStatusLoaded ? (
          twitchStatus.connected ? (
            <Button
              disabled={isTwitchBusy}
              onClick={onDisconnectTwitch}
              size="small"
              sx={{ marginRight: 1 }}
              variant="contained"
            >
              {isTwitchBusy ? 'Disconnecting...' : 'Disconnect'}
            </Button>
          ) : (
            <Button
              onClick={onConnectTwitch}
              size="small"
              sx={{ marginRight: 1 }}
              variant="contained"
            >
              Connect Twitch
            </Button>
          )
        ) : (
          <Button disabled size="small" sx={{ marginRight: 1 }} variant="contained">
            Loading...
          </Button>
        )}
        <Button disabled={isBusy} onClick={onSignOut} size="small" variant="outlined">
          Sign out
        </Button>
      </Box>

      {/* Mobile: collapsed menu */}
      <Box sx={{ display: { md: 'none', xs: 'flex' } }}>
        <IconButton aria-label="Menu" onClick={handleMenuOpen} size="large">
          <Ellipsis size={18} />
        </IconButton>
        <Menu
          anchorEl={anchorElement}
          anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
          onClose={handleMenuClose}
          open={isMenuOpen}
          transformOrigin={{ horizontal: 'right', vertical: 'top' }}
        >
          <MenuItem
            disabled={!isTwitchStatusLoaded || isTwitchBusy}
            onClick={handleTwitchMenuClick}
          >
            {getTwitchMenuText()}
          </MenuItem>
          <MenuItem disabled={isBusy} onClick={handleSignOutClick}>
            Sign out
          </MenuItem>
        </Menu>
      </Box>
    </Box>
  );
};
