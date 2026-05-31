import { useCallback, useEffect, useRef, type ReactElement, type ReactNode } from 'react';
import { useRouter } from '../../hooks/useRouter';
import AppVersion from './app-version';

const POPSTATE_TYPE = 'popstate';

interface TwitchLayoutProps {
  children: ReactNode;
}

/**
 * Twitch layout component with theme and focus management.
 * Ported from Svelte twitch-layout.svelte.
 */
export default function TwitchLayout({ children }: TwitchLayoutProps): ReactElement {
  const mainElementRef = useRef<HTMLElement>(null);
  const { afterNavigate } = useRouter();

  // Set Twitch theme on body on mount, cleanup on unmount
  useEffect(() => {
    document.body.dataset.theme = 'twitch';

    return (): void => {
      delete document.body.dataset.theme;
    };
  }, []);

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
    <div className="twitch-app">
      <main
        ref={mainElementRef}
        className="twitch-main"
        tabIndex={-1}
        aria-label="Twitch Relay main content"
      >
        {children}
      </main>

      <AppVersion />
    </div>
  );
}
