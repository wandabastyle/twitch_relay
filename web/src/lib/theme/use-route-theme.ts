import { useMemo } from 'react';
import { useRouter } from '../../hooks/use-router';
import { twitchTheme } from './twitch-theme';
import { youtubeTheme } from './youtube-theme';
import type { AppTheme } from './theme-types';
import type { Theme } from '@mui/material/styles';

export const useRouteTheme = (): { theme: AppTheme; muiTheme: Theme } => {
  const { page } = useRouter();
  const { path } = page;

  return useMemo(() => {
    // Determine theme based on route
    if (path.startsWith('/youtube')) {
      return { theme: 'youtube' as AppTheme, muiTheme: youtubeTheme };
    }
    // Default to Twitch theme for all other routes
    return { theme: 'twitch' as AppTheme, muiTheme: twitchTheme };
  }, [path]);
};
