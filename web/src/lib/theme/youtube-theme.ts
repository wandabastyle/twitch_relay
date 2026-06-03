import { createTheme } from '@mui/material/styles';

export const youtubeTheme = createTheme({
  palette: {
    mode: 'dark',
    background: {
      default: '#2a171d',
      paper: '#462a35',
    },
    primary: {
      main: '#ff0033',
      light: '#cc0029',
      dark: '#cc0029',
      contrastText: '#ffffff',
    },
    secondary: {
      main: '#c099ff',
      light: '#d4b8ff',
      dark: '#9a7acc',
      contrastText: '#2a171d',
    },
    success: {
      main: '#4caf50',
      light: '#81c784',
      dark: '#388e3c',
      contrastText: '#ffffff',
    },
    warning: {
      main: '#ffb74d',
      light: '#ffcc80',
      dark: '#f57c00',
      contrastText: '#2a171d',
    },
    error: {
      main: '#ff5252',
      light: '#ff8a80',
      dark: '#c62828',
      contrastText: '#ffffff',
    },
    text: {
      primary: '#c8d3f5',
      secondary: '#a9b8e8',
    },
    divider: '#7b3f52',
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
