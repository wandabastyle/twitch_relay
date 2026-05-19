/**
 * Client-side router for Svelte application
 * Replaces SvelteKit's file-based routing with reactive Svelte 5 runes
 *
 * Usage:
 * 1. Call initRouter() in your root component/layout
 * 2. Use reactive exports like currentPath, currentParams for state
 * 3. Use navigate() and goBack() for navigation
 * 4. Use page store for SvelteKit-compatible $page access
 */
import type { Readable } from 'svelte/store';
import {
  matchRoute,
  parseQueryParams,
  ROUTES,
  type RouteParams,
  type QueryParams,
  type HistoryState,
  type PageStore,
  type NavigationType,
  type NavigationOptions,
} from './routes';

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

function isClient(): boolean {
  return typeof window !== 'undefined';
}

/**
 * Initializes the router state from the current URL.
 */
function initializeState(): void {
  const initialPath = isClient() ? window.location.pathname : '/';
  const initialUrl = isClient() ? new URL(window.location.href) : new URL('http://localhost/');
  const initialMatch = matchRoute(initialPath);

  path = initialPath;
  url = initialUrl;
  params = initialMatch?.params ?? {};
  query = {};
  historyState = null;
  navigationType = 'goto';
  redirect = initialPath === '/' ? '/twitch' : null;
}

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
export function initRouter(): void {
  if (isInitialized) return;

  initializeState();
  isInitialized = true;

  if (isClient()) {
    // Initialize from current URL
    updateFromUrl();

    // Handle browser back/forward buttons
    window.addEventListener('popstate', () => {
      navigationType = 'popstate';
      updateFromUrl();
    });
  }
}

/**
 * Updates router state from current browser URL.
 */
function updateFromUrl(): void {
  if (!isClient()) return;

  const currentUrl = new URL(window.location.href);
  url = currentUrl;
  path = currentUrl.pathname;
  query = parseQueryParams(currentUrl.searchParams);

  // Get history state
  historyState = window.history.state as HistoryState | null;

  // Match route and extract params
  const match = matchRoute(path);
  if (match) {
    params = match.params;
  } else {
    params = {};
  }

  // Handle redirects
  handleRedirects();
}

/**
 * Handles special redirect cases.
 */
function handleRedirects(): void {
  if (path === '/') {
    redirect = '/twitch';
  } else {
    redirect = null;
  }
}

// ============================================================================
// Reactive State Exports
// ============================================================================

/** Current path as reactive state */
export function getCurrentPath(): string {
  if (!isInitialized) return '/';
  return path;
}

/** Current route params as reactive state */
export function getCurrentParams(): RouteParams {
  if (!isInitialized) return {};
  return params;
}

/** Current query params as reactive state */
export function getCurrentQuery(): QueryParams {
  if (!isInitialized) return {};
  return query;
}

/** Current history state */
export function getCurrentState(): HistoryState | null {
  if (!isInitialized) return null;
  return historyState;
}

/** Current URL */
export function getCurrentUrl(): URL {
  if (!isInitialized) return new URL('http://localhost/');
  return url;
}

/** Navigation type */
export function getNavigationType(): NavigationType {
  if (!isInitialized) return 'goto';
  return navigationType;
}

/** Current redirect target if any */
export function getCurrentRedirect(): string | null {
  if (!isInitialized) return null;
  return redirect;
}

// ============================================================================
// Navigation Functions
// ============================================================================

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
export function navigate(targetPath: string, options: NavigationOptions = {}): void {
  if (!isClient()) return;

  const { replace = false, state } = options;

  if (isInitialized) {
    navigationType = replace ? 'replace' : 'goto';
  }

  if (replace) {
    window.history.replaceState(state, '', targetPath);
  } else {
    window.history.pushState(state, '', targetPath);
  }

  // Update state from new URL
  updateFromUrl();
}

/**
 * Goes back in browser history.
 * Replacement for window.history.back() with state tracking.
 *
 * @example
 * ```ts
 * goBack();
 * ```
 */
export function goBack(): void {
  if (!isClient()) return;

  if (window.history.length > 1) {
    window.history.back();
  } else {
    // Fallback: navigate to home
    navigate('/twitch');
  }
}

// ============================================================================
// Page Store (SvelteKit-compatible)
// ============================================================================

/**
 * Gets the current page state for reactive access.
 * Mirrors SvelteKit's $page store structure.
 *
 * @returns Current page state object
 */
export function getPage(): PageStore {
  if (!isInitialized) {
    return {
      path: '/',
      params: {},
      query: {},
      state: null,
      url: new URL('http://localhost/'),
    };
  }

  return {
    path,
    params,
    query,
    state: historyState,
    url,
  };
}

/**
 * Creates a Svelte store compatible with SvelteKit's $page store.
 * Use this for gradual migration from SvelteKit.
 *
 * @example
 * ```svelte
 * <script>
 *   import { page } from '$lib/router/router.svelte';
 *   // Now you can use $page.params.login, $page.query.channel_login, etc.
 * </script>
 * ```
 */
export const page: Readable<PageStore> = {
  subscribe(fn) {
    // Use $effect to react to changes
    $effect(() => {
      fn(getPage());
    });
    return () => {
      // Cleanup handled by Svelte
    };
  },
};

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
export function useRedirect(): void {
  $effect(() => {
    if (isInitialized && redirect) {
      navigate(redirect, { replace: true });
    }
  });
}

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
export function isPopStateNavigation(): boolean {
  if (!isInitialized) return false;
  return navigationType === 'popstate';
}

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
export function afterNavigate(
  callback: (navigation: { type: NavigationType; from?: string; to: string }) => void,
): void {
  let previousPath = isInitialized ? path : '/';

  $effect(() => {
    if (!isInitialized) return;

    const currentPath = path;
    if (currentPath !== previousPath) {
      callback({
        type: navigationType,
        from: previousPath,
        to: currentPath,
      });
      previousPath = currentPath;
    }
  });
}

/**
 * Gets the youtubeReturnUrl from history state if present.
 * Used by YouTube watch pages to know where to return.
 *
 * @returns The return URL or undefined
 */
export function getYouTubeReturnUrl(): string | undefined {
  if (!isInitialized) return undefined;
  return historyState?.youtubeReturnUrl;
}

// Re-export types and utilities from routes module for convenience
export { ROUTES };
