import type {
  HistoryState,
  NavigationOptions,
  NavigationType,
  PageStore,
  QueryParams,
  RouteParams,
} from './routes';
import { ROUTES, matchRoute, parseQueryParams } from './routes';

export * from './routes';

// ============================================================================
// Module-level State Declarations (Svelte 5 runes)
// ============================================================================

/** Module-level router state using Svelte 5 runes */
let path = $state<string>('/');
let params = $state<RouteParams>({});
let query = $state<QueryParams>({});
let historyState = $state<HistoryState | null>(null);
let navigationType = $state<NavigationType>('goto');
let url = $state<URL>(new URL('http://localhost/'));
let redirect = $state<string | null>(null);

let isInitialized = $state(false);

const isClient = (): boolean => typeof globalThis.window !== 'undefined';

const getInitialPath = (): string => {
  if (isClient()) {
    return globalThis.window.location.pathname;
  }
  return '/';
};

const getInitialUrl = (): URL => {
  if (isClient()) {
    return new URL(globalThis.window.location.href);
  }
  return new URL('http://localhost/');
};

const getInitialRedirect = (initialPath: string): string | null => {
  if (initialPath === '/') {
    return '/twitch';
  }
  return null;
};

/**
 * Initializes the router state from the current URL.
 */
const initializeState = (): void => {
  const initialPath = getInitialPath();
  const initialUrl = getInitialUrl();
  const initialMatch = matchRoute(initialPath);

  path = initialPath;
  url = initialUrl;
  params = initialMatch?.params ?? {};
  query = {};
  historyState = null;
  navigationType = 'goto';
  redirect = getInitialRedirect(initialPath);
};

/**
 * Initializes the router. Must be called once before using any router functions.
 * This should be called in your root layout or App component.
 *
 * @example
 * ```svelte
 * <script>
 *   import { initRouter } from '$lib/router/router.svelte';
 *   initRouter();
 * </script>
 * ```
 */
export const initRouter = (): void => {
  if (isInitialized) {
    return;
  }

  initializeState();
  isInitialized = true;

  if (isClient()) {
    // Initialize from current URL
    updateFromUrl();

    // Handle browser back/forward buttons
    globalThis.window.addEventListener('popstate', () => {
      navigationType = 'popstate';
      updateFromUrl();
    });
  }
};

/**
 * Updates router state from current browser URL.
 */
const updateFromUrl = (): void => {
  if (!isClient()) {
    return;
  }

  const currentUrl = new URL(globalThis.window.location.href);
  url = currentUrl;
  path = currentUrl.pathname;
  query = parseQueryParams(currentUrl.searchParams);

  // Get history state
  historyState = globalThis.window.history.state as HistoryState | null;

  // Match route and extract params
  const matchResult = matchRoute(path);
  params = matchResult ? matchResult.params : {};

  // Handle redirects
  handleRedirects();
};

/**
 * Handles special redirect cases.
 */
const handleRedirects = (): void => {
  redirect = path === '/' ? '/twitch' : null;
};

// ============================================================================
// Reactive State Exports
// ============================================================================

/** Current path as reactive state */
export const getCurrentPath = (): string => {
  if (!isInitialized) {
    return '/';
  }
  return path;
};

/** Current route params as reactive state */
export const getCurrentParams = (): RouteParams => {
  if (!isInitialized) {
    return {};
  }
  return params;
};

/** Current query params as reactive state */
export const getCurrentQuery = (): QueryParams => {
  if (!isInitialized) {
    return {};
  }
  return query;
};

/** Current history state */
export const getCurrentState = (): HistoryState | null => {
  if (!isInitialized) {
    return null;
  }
  return historyState;
};

/** Current URL */
export const getCurrentUrl = (): URL => {
  if (!isInitialized) {
    return new URL('http://localhost/');
  }
  return url;
};

/** Navigation type */
export const getNavigationType = (): NavigationType => {
  if (!isInitialized) {
    return 'goto';
  }
  return navigationType;
};

/** Current redirect target if any */
export const getCurrentRedirect = (): string | null => {
  if (!isInitialized) {
    return null;
  }
  return redirect;
};

// ============================================================================
// Navigation Functions
// ============================================================================

const getNavigationTypeFromOptions = (replace: boolean): NavigationType => {
  if (replace) {
    return 'replace';
  }
  return 'goto';
};

/**
 * Navigates to a new path. Replacement for SvelteKit's goto().
 *
 * @param path - The path to navigate to
 * @param options - Navigation options including replace and state
 *
 * @example
 * ```ts
 * navigate('/twitch');
 * navigate('/youtube/channel/UCxxx');
 * navigate('/youtube/watch/abc', { state: { youtubeReturnUrl: '/youtube' } });
 * navigate('/twitch/recordings/play', { replace: true });
 * ```
 */
export const navigate = (targetPath: string, options: NavigationOptions = {}): void => {
  if (!isClient()) {
    return;
  }

  const { replace = false, state } = options;

  if (isInitialized) {
    navigationType = getNavigationTypeFromOptions(replace);
  }

  if (replace) {
    globalThis.window.history.replaceState(state, '', targetPath);
  } else {
    globalThis.window.history.pushState(state, '', targetPath);
  }

  // Update state from new URL
  updateFromUrl();
};

/**
 * Goes back in browser history.
 * Replacement for globalThis.window.history.back() with state tracking.
 *
 * @example
 * ```ts
 * goBack();
 * ```
 */
const MIN_HISTORY_LENGTH = 1;

export const goBack = (): void => {
  if (!isClient()) {
    return;
  }

  if (globalThis.window.history.length > MIN_HISTORY_LENGTH) {
    globalThis.window.history.back();
  } else {
    // Fallback: navigate to home
    navigate('/twitch');
  }
};

// ============================================================================
// Reactive State Exports (Svelte 5 runes - use directly in components)
// ============================================================================

/**
 * Reactive router state exports.
 * These are reactive Svelte 5 runes that can be used directly in components.
 *
 * @example
 * ```svelte
 * <script>
 *   import { page, path, params, query } from '$lib/router/router.svelte';
 *
 *   // Access reactive state directly
 *   console.log(page.params.login);
 *   console.log(path);
 * </script>
 * ```
 */

/** Current URL - reactive state */
export const currentUrl = {
  get value() {
    return url;
  },
};

/** Current path - reactive state */
export const currentPath = {
  get value() {
    return path;
  },
};

/** Current route params - reactive state */
export const currentParams = {
  get value() {
    return params;
  },
};

/** Current query params - reactive state */
export const currentQuery = {
  get value() {
    return query;
  },
};

/** Current history state - reactive state */
export const currentState = {
  get value() {
    return historyState;
  },
};

/**
 * Page store object for SvelteKit-compatible access.
 * Use `page.params`, `page.query`, etc. directly in reactive contexts.
 * No need for $ prefix - access properties directly.
 */
export const page: PageStore = {
  get params() {
    return params;
  },
  get path() {
    return path;
  },
  get query() {
    return query;
  },
  get state() {
    return historyState;
  },
  get url() {
    return url;
  },
};

// ============================================================================
// Navigation Functions
// ============================================================================

// ============================================================================
// Route Guards and Hooks
// ============================================================================

/**
 * Hook to handle redirects reactively in components.
 * Call this in $effect for automatic redirect handling.
 *
 * @example
 * ```svelte
 * <script>
 *   import { useRedirect } from '$lib/router/router.svelte';
 *
 *   $effect(() => {
 *     useRedirect();
 *   });
 * </script>
 * ```
 */
export const useRedirect = (): void => {
  $effect(() => {
    if (isInitialized && redirect) {
      navigate(redirect, { replace: true });
    }
  });
};

/**
 * Hook to check if navigation was a back/forward navigation.
 * Useful for scroll position restoration or conditional loading.
 *
 * @example
 * ```svelte
 * <script>
 *   import { isPopStateNavigation } from '$lib/router/router.svelte';
 *
 *   $effect(() => {
 *     if (isPopStateNavigation()) {
 *       // Restore scroll position
 *     }
 *   });
 * </script>
 * ```
 */
export const isPopStateNavigation = (): boolean => {
  if (!isInitialized) {
    return false;
  }
  return navigationType === 'popstate';
};

const getPreviousPath = (): string => {
  if (isInitialized) {
    return path;
  }
  return '/';
};

/**
 * Hook to run code after navigation completes.
 * Similar to SvelteKit's afterNavigate.
 *
 * @param callback - Function to run after navigation
 *
 * @example
 * ```svelte
 * <script>
 *   import { afterNavigate } from '$lib/router/router.svelte';
 *
 *   afterNavigate((navigation) => {
 *     if (navigation.type === 'goto') {
 *       // Scroll to top
 *     }
 *   });
 * </script>
 * ```
 */
export const afterNavigate = (
  callback: (navigation: { from?: string; to: string; type: NavigationType }) => void,
): void => {
  let previousPath = getPreviousPath();

  $effect(() => {
    if (!isInitialized) {
      return;
    }

    const currentPath = path;
    if (currentPath !== previousPath) {
      callback({
        from: previousPath,
        to: currentPath,
        type: navigationType,
      });
      previousPath = currentPath;
    }
  });
};

/**
 * Gets the youtubeReturnUrl from history state if present.
 * Used by YouTube watch pages to know where to return.
 *
 * @returns The return URL or undefined
 */
export const getYouTubeReturnUrl = (): string | undefined => {
  if (!isInitialized) {
    return undefined;
  }
  return historyState?.youtubeReturnUrl;
};

// Re-export types and utilities from routes module for convenience
export { ROUTES };
