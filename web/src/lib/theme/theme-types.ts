import type { Theme } from '@mui/material/styles';

export type AppTheme = 'twitch' | 'youtube';

export interface ThemeContextValue {
  theme: AppTheme;
  muiTheme: Theme;
}
