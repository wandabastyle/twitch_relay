import { useCallback, useEffect, useRef, useState } from 'react';
import {
  type NavigationOptions,
  type NavigationType,
  type PageStore,
  normalizeHistoryState,
  ROUTES,
  matchRoute,
  parseHistoryState,
  parseQueryParams,
} from '../router/routes';

const MIN_HISTORY_LENGTH = 1;

interface UseRouterReturn {
  page: PageStore;
  navigate: (targetPath: string, options?: NavigationOptions) => void;
  goBack: () => void;
  afterNavigate: (callback: NavigationCallback) => () => void;
}

type NavigationCallback = (
  navigation: Readonly<{ from?: string; to: string; type: NavigationType }>,
) => void;

const isClient = (): boolean => typeof window !== 'undefined';

// Store callbacks for afterNavigate
const afterNavigateCallbacks = new Set<NavigationCallback>();

// AfterNavigate callback registration function (outside hook)
const registerAfterNavigate = (callback: NavigationCallback): (() => void) => {
  afterNavigateCallbacks.add(callback);
  return () => {
    afterNavigateCallbacks.delete(callback);
  };
}

/**
 * Custom hook for SPA routing with history state management.
 * Ported from Svelte router to React.
 */
export const useRouter = (): UseRouterReturn => {
  // State for page store
  const [page, setPage] = useState<PageStore>(() => {
    if (!isClient()) {
      return {
        params: {},
        path: '/',
        query: {},
        state: null,
        url: new URL('http://localhost/'),
      };
    }
    const currentUrl = new URL(window.location.href);
    const currentPath = currentUrl.pathname;
    const matchResult = matchRoute(currentPath);

    return {
      params: matchResult === null ? {} : matchResult.params,
      path: currentPath,
      query: parseQueryParams(currentUrl.searchParams),
      state: parseHistoryState(window.history.state),
      url: currentUrl,
    };
  });

  // Navigation type tracking
  const navigationTypeRef = useRef<NavigationType>('goto');
  const previousPathRef = useRef<string>(page.path);

  // Update page from current URL
  const updateFromUrl = useCallback((): void => {
    if (!isClient()) {
      return;
    }
    const currentUrl = new URL(window.location.href);
    const currentPath = currentUrl.pathname;
    const matchResult = matchRoute(currentPath);

    setPage({
      params: matchResult === null ? {} : matchResult.params,
      path: currentPath,
      query: parseQueryParams(currentUrl.searchParams),
      state: parseHistoryState(window.history.state),
      url: currentUrl,
    });
  }, []);

  // Navigate to a new path
  const navigate = useCallback(
    (targetPath: string, options: NavigationOptions = {}): void => {
      if (!isClient()) {
        return;
      }

      const { replace = false, state } = options;

      navigationTypeRef.current = replace ? 'replace' : 'goto';

      if (replace) {
        window.history.replaceState(normalizeHistoryState(state), '', targetPath);
      } else {
        window.history.pushState(normalizeHistoryState(state), '', targetPath);
      }

      // Dispatch popstate event to trigger route updates
      window.dispatchEvent(new PopStateEvent('popstate'));
    },
    [],
  );

  // Go back in history
  const goBack = useCallback((): void => {
    if (!isClient()) {
      return;
    }

    if (window.history.length > MIN_HISTORY_LENGTH) {
      window.history.back();
    } else {
      navigate('/twitch');
    }
  }, [navigate]);

  // Listen to popstate events
  useEffect(() => {
    if (!isClient()) {
      return undefined;
    }

    const handlePopState = (): void => {
      navigationTypeRef.current = 'popstate';
      updateFromUrl();
    };

    window.addEventListener('popstate', handlePopState);
    return (): void => {
      window.removeEventListener('popstate', handlePopState);
    };
  }, [updateFromUrl]);

  // Handle redirect from index to /twitch
  useEffect(() => {
    if (page.path === '/') {
      navigate('/twitch', { replace: true });
    }
  }, [page.path, navigate]);

  // Track navigation changes and notify callbacks
  useEffect(() => {
    if (page.path !== previousPathRef.current) {
      const from = previousPathRef.current;
      const to = page.path;
      const type = navigationTypeRef.current;

      // Notify all registered callbacks
      for (const callback of afterNavigateCallbacks) {
        callback({ from, to, type });
      }

      previousPathRef.current = page.path;
    }
  }, [page]);

  return {
    afterNavigate: registerAfterNavigate,
    goBack,
    navigate,
    page,
  };
}

/**
 * Hook to get the YouTube return URL from history state
 */
export const useYouTubeReturnUrl = (): string | undefined => {
  const [returnUrl, setReturnUrl] = useState<string | undefined>();

  useEffect(() => {
    if (typeof window === 'undefined') {
      return undefined;
    }
    const state = parseHistoryState(window.history.state);
    setReturnUrl(state?.youtubeReturnUrl);

    const handlePopState = (): void => {
      const newState = parseHistoryState(window.history.state);
      setReturnUrl(newState?.youtubeReturnUrl);
    };

    window.addEventListener('popstate', handlePopState);
    return (): void => {
      window.removeEventListener('popstate', handlePopState);
    };
  }, []);

  return returnUrl;
}

/**
 * Hook to check if current navigation is from popstate (back/forward)
 */
export const useIsPopStateNavigation = (): boolean => {
  const [isPopState, setIsPopState] = useState(false);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return undefined;
    }

    const handlePopState = (): void => {
      setIsPopState(true);
    };

    const handlePushState = (): void => {
      setIsPopState(false);
    };

    window.addEventListener('popstate', handlePopState);

    // Listen for custom pushstate events
    window.addEventListener('pushstate', handlePushState as EventListener);

    return (): void => {
      window.removeEventListener('popstate', handlePopState);
      window.removeEventListener('pushstate', handlePushState as EventListener);
    };
  }, []);

  return isPopState;
}

export { ROUTES };
