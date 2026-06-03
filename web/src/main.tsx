import './lib/styles/app.css';

import { CssBaseline, ThemeProvider as MuiThemeProvider } from '@mui/material';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './app';
import { ThemeProvider } from './lib/theme/theme-context';
import { useRouteTheme } from './lib/theme/use-route-theme';

const ThemedApp = (): React.ReactElement => {
  const { theme, muiTheme } = useRouteTheme();

  return (
    <ThemeProvider theme={theme} muiTheme={muiTheme}>
      <MuiThemeProvider theme={muiTheme}>
        <CssBaseline />
        <App />
      </MuiThemeProvider>
    </ThemeProvider>
  );
};

const appElement = document.querySelector('#app');

if (!appElement) {
  throw new Error('Failed to find #app element');
}

const root = createRoot(appElement);

root.render(
  <StrictMode>
    <ThemedApp />
  </StrictMode>,
);
