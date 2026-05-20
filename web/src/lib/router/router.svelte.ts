import {
  type HistoryState,
  type NavigationOptions,
  type NavigationType,
  type PageStore,
  type QueryParams,
  type RouteParams,
  normalizeHistoryState,
  ROUTES,
  matchRoute,
  parseHistoryState,
  parseQueryParams,
} from './routes';

const MIN_HISTORY_LENGTH = 1;

export * from './routes';

let path = $state<string>('/');
let params = $state<RouteParams>({});
let query = $state<QueryParams>({});
let historyState = $state<HistoryState | null>(null);
let navigationType = $state<NavigationType>('goto');
let url = $state<URL>(new URL('http://localhost/'));
let redirect = $state<string | null>(null);

let isInitialized = $state(false);

const isClient = (): boolean => 'window' in globalThis;

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

const initializeState = (): void => {
  const initialPath = getInitialPath();
  const initialUrl = getInitialUrl();
  const initialMatch = matchRoute(initialPath);

  path = initialPath;
  url = initialUrl;
  params = initialMatch === null ? {} : initialMatch.params;
  query = {};
  historyState = null;
  navigationType = 'goto';
  redirect = getInitialRedirect(initialPath);
};

const applyCurrentLocation = (): void => {
  const currentUrl = new URL(globalThis.window.location.href);
  const currentPath = currentUrl.pathname;

  url = currentUrl;
  path = currentPath;
  query = parseQueryParams(currentUrl.searchParams);
  historyState = parseHistoryState(globalThis.window.history.state);

  const matchResult = matchRoute(currentPath);
  params = matchResult === null ? {} : matchResult.params;
  redirect = currentPath === '/' ? '/twitch' : null;
};

const updateFromUrl = (): void => {
  if (!isClient()) {
    return;
  }
  applyCurrentLocation();
};

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

export const getCurrentPath = (): string => {
  if (!isInitialized) {
    return '/';
  }
  return path;
};

export const getCurrentParams = (): RouteParams => {
  if (!isInitialized) {
    return {};
  }
  return params;
};

export const getCurrentQuery = (): QueryParams => {
  if (!isInitialized) {
    return {};
  }
  return query;
};

export const getCurrentState = (): HistoryState | null => {
  if (!isInitialized) {
    return null;
  }
  return historyState;
};

export const getCurrentUrl = (): URL => {
  if (!isInitialized) {
    return new URL('http://localhost/');
  }
  return url;
};

export const getNavigationType = (): NavigationType => {
  if (!isInitialized) {
    return 'goto';
  }
  return navigationType;
};

export const getCurrentRedirect = (): string | null => {
  if (!isInitialized) {
    return null;
  }
  return redirect;
};

const getNavigationTypeFromOptions = (replace: boolean): NavigationType => {
  if (replace) {
    return 'replace';
  }
  return 'goto';
};

export const navigate = (
  targetPath: Readonly<string>,
  options: Readonly<NavigationOptions> = {},
): void => {
  if (!isClient()) {
    return;
  }

  const { replace = false, state } = options;

  if (isInitialized) {
    navigationType = getNavigationTypeFromOptions(replace);
  }

  if (replace) {
    globalThis.window.history.replaceState(normalizeHistoryState(state), '', targetPath);
  } else {
    globalThis.window.history.pushState(normalizeHistoryState(state), '', targetPath);
  }

  updateFromUrl();
};

export const goBack = (): void => {
  if (!isClient()) {
    return;
  }

  if (globalThis.window.history.length > MIN_HISTORY_LENGTH) {
    globalThis.window.history.back();
  } else {
    navigate('/twitch');
  }
};
export const currentUrl: { get value(): URL } = {
  get value(): URL {
    return url;
  },
};

export const currentPath: { get value(): string } = {
  get value(): string {
    return path;
  },
};

export const currentParams: { get value(): RouteParams } = {
  get value(): RouteParams {
    return params;
  },
};

export const currentQuery: { get value(): QueryParams } = {
  get value(): QueryParams {
    return query;
  },
};

export const currentState: { get value(): HistoryState | null } = {
  get value(): HistoryState | null {
    return historyState;
  },
};

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

export const useRedirect = (): void => {
  $effect(() => {
    if (isInitialized && redirect !== null && redirect !== '') {
      navigate(redirect, { replace: true });
    }
  });
};

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

type NavigationCallback = (
  navigation: Readonly<{ from?: string; to: string; type: NavigationType }>,
) => void;

export const afterNavigate = (callback: NavigationCallback): void => {
  let previousPath = getPreviousPath();

  $effect(() => {
    if (!isInitialized) {
      return;
    }

    const nextPath = path;
    if (nextPath !== previousPath) {
      callback({
        from: previousPath,
        to: nextPath,
        type: navigationType,
      });
      previousPath = nextPath;
    }
  });
};

export const getYouTubeReturnUrl = (): string | undefined => {
  if (!isInitialized || historyState === null) {
    return undefined;
  }
  return historyState.youtubeReturnUrl;
};

export { ROUTES };
