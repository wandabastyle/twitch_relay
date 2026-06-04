import { AppBar, Toolbar, Typography, IconButton, Box } from '@mui/material';
import { ArrowLeftRight } from 'lucide-react';
import type { ReactElement, ReactNode } from 'react';

interface RelayHeaderProps {
  eyebrow: string;
  title: string;
  subtitleText?: string;
  subtitleSnippet?: ReactNode;
  onToggle: () => void;
  toggleLabel: string;
  children?: ReactNode;
}

export const RelayHeader = ({
  children,
  eyebrow,
  onToggle,
  subtitleSnippet,
  subtitleText,
  title,
  toggleLabel,
}: RelayHeaderProps): ReactElement => (
  <AppBar
    color="default"
    elevation={0}
    position="static"
    sx={{ backgroundColor: 'transparent', padding: 2 }}
  >
    <Toolbar disableGutters sx={{ alignItems: 'flex-start', flexDirection: 'column' }}>
      <Box sx={{ alignItems: 'center', display: 'flex' }}>
        <Typography
          variant="subtitle2"
          component="p"
          sx={{ color: 'text.secondary', mr: 1, textTransform: 'uppercase' }}
        >
          {eyebrow}
        </Typography>
        <IconButton
          onClick={onToggle}
          aria-label={toggleLabel}
          title={toggleLabel}
          edge="start"
          size="large"
          sx={{ alignItems: 'center', display: 'flex', marginRight: 1, padding: 0 }}
        >
          <Typography variant="h5" component="h1" sx={{ mr: 0.5 }}>
            {title}
          </Typography>
          <Box component="span" aria-hidden="true" sx={{ alignItems: 'center', display: 'flex' }}>
            <ArrowLeftRight size={14} />
          </Box>
        </IconButton>
      </Box>
      {(subtitleText !== undefined && subtitleText !== '') ||
      (subtitleSnippet !== undefined && subtitleSnippet !== null) ? (
        <Typography variant="body2" component="p" sx={{ mt: 1 }}>
          {subtitleText ?? subtitleSnippet}
        </Typography>
      ) : null}
    </Toolbar>
    {children}
  </AppBar>
);
