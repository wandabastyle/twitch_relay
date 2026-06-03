import { createTheme } from '@mui/material/styles';

export const twitchTheme = createTheme({
  palette: {
    mode: 'dark',
    background: {
      default: '#1e2030',
      paper: '#2f334d',
    },
    primary: {
      main: '#82aaff',
      light: '#a8c5ff',
      dark: '#5c7dbf',
      contrastText: '#1e2030',
    },
    secondary: {
      main: '#c099ff',
      light: '#d4b8ff',
      dark: '#9a7acc',
      contrastText: '#1e2030',
    },
    success: {
      main: '#c3e88d',
      light: '#d4eda8',
      dark: '#8fa363',
      contrastText: '#1e2030',
    },
    warning: {
      main: '#ffc777',
      light: '#ffd49a',
      dark: '#b28c53',
      contrastText: '#1e2030',
    },
    error: {
      main: '#ff757f',
      light: '#ff9aa1',
      dark: '#b35259',
      contrastText: '#1e2030',
    },
    text: {
      primary: '#c8d3f5',
      secondary: '#a9b8e8',
    },
    divider: '#444a73',
  },
  typography: {
    fontFamily: '"IBM Plex Sans", "Noto Sans", sans-serif',
    h1: {
      fontSize: 'clamp(1.45rem, 4vw, 1.9rem)',
      lineHeight: 1.1,
      fontWeight: 600,
    },
    h2: {
      fontSize: '1.25rem',
      lineHeight: 1.2,
      fontWeight: 600,
    },
    h3: {
      fontSize: '1.1rem',
      lineHeight: 1.25,
      fontWeight: 600,
    },
    body1: {
      fontSize: '0.95rem',
      lineHeight: 1.5,
    },
    body2: {
      fontSize: '0.85rem',
      lineHeight: 1.5,
    },
    button: {
      fontSize: '0.85rem',
      fontWeight: 600,
      textTransform: 'none',
    },
  },
  shape: {
    borderRadius: 12,
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          borderRadius: '0.6rem',
          minHeight: '2.35rem',
        },
      },
    },
    MuiIconButton: {
      styleOverrides: {
        root: {
          borderRadius: '0.5rem',
        },
      },
    },
    MuiTextField: {
      styleOverrides: {
        root: {
          '& .MuiOutlinedInput-root': {
            borderRadius: '0.6rem',
          },
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: {
          borderRadius: '0.75rem',
          backgroundImage: 'none',
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
        },
      },
    },
  },
});
