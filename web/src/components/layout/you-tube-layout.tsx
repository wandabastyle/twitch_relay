import Box from '@mui/material/Box';
import { useCallback, useEffect, useRef, type ReactElement, type ReactNode } from 'react';
import { useRouter } from '../../hooks/use-router';
import AppVersion from './app-version';

const POPSTATE_TYPE = 'popstate';

interface YouTubeLayoutProps {
  children: ReactNode;
}

/**
 * YouTube layout component with focus management.
 * Ported from Svelte you-tube-layout.svelte.
 */
export default function YouTubeLayout({ children }: YouTubeLayoutProps): ReactElement {
  const mainElementRef = useRef<HTMLElement>(null);
  const { afterNavigate } = useRouter();

  // Focus management: only on forward navigations, not back/forward
  const handleNavigation = useCallback(
    (navigation: { from?: string; to: string; type: string }) => {
      // Skip focus management for popstate (back/forward) to preserve scroll position
      if (navigation.type === POPSTATE_TYPE) {
        return;
      }

      // Focus the main container for keyboard navigation, but don't scroll
      if (mainElementRef.current) {
        mainElementRef.current.focus({ preventScroll: true });
      }
    },
    [],
  );

  // Register navigation callback
  useEffect(() => {
    const cleanup = afterNavigate(handleNavigation);
    return cleanup;
  }, [afterNavigate, handleNavigation]);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      <Box
        component="main"
        ref={mainElementRef}
        tabIndex={-1}
        aria-label="YouTube Relay main content"
        sx={{ flexGrow: 1, outline: 'none' }}
      >
        {children}
      </Box>

      <AppVersion />
    </Box>
  );
}
