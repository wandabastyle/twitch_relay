import { createContext, useContext, useMemo, type ReactElement, type ReactNode } from 'react';
import type { Theme } from '@mui/material/styles';
import type { AppTheme } from './theme-types';

interface ThemeContextValue {
  theme: AppTheme;
  muiTheme: Theme;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

interface ThemeProviderProps {
  theme: AppTheme;
  muiTheme: Theme;
  children: ReactNode;
}

export const ThemeProvider = ({
  theme,
  muiTheme,
  children,
}: ThemeProviderProps): ReactElement => {
  const value = useMemo(
    () => ({ theme, muiTheme }),
    [theme, muiTheme]
  );

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useAppTheme = (): ThemeContextValue => {
  const context = useContext(ThemeContext);
  if (context === null) {
    throw new Error('useAppTheme must be used within a ThemeProvider');
  }
  return context;
};

export const useThemeName = (): AppTheme => {
  const context = useContext(ThemeContext);
  if (context === null) {
    throw new Error('useThemeName must be used within a ThemeProvider');
  }
  return context.theme;
};
