import { getSessionState, login, logout } from '$lib/api-client';
import { navigate } from '$lib/router/router.svelte';
import { readJsError } from '$lib/home/errors';
import type { ChannelsController } from './channels-controller.core.svelte';

export type AuthMode = 'authenticated' | 'checking' | 'unauthenticated';

export interface AuthControllerDeps {
  channelsController: ChannelsController;
  onAuthenticated: () => Promise<void>;
  setError: (message: string | null) => void;
}

type PreventDefaultEvent = Readonly<{ preventDefault: () => void }>;

export interface AuthController {
  accessCode: string;
  authMode: AuthMode;
  initialize: () => Promise<void>;
  isBusy: boolean;
  setAccessCode: (value: string) => void;
  signOut: () => Promise<void>;
  submitLogin: (event: PreventDefaultEvent) => Promise<void>;
}

const REQUIRED_CODE_ERROR = 'access code is required';

const normalizeCode = (value: string): string | null => {
  const normalized = value.trim();
  return normalized === '' ? null : normalized;
};

const createInitialize =
  (
    deps: Readonly<AuthControllerDeps>,
    setAuthMode: (mode: AuthMode) => void,
  ): (() => Promise<void>) =>
  async (): Promise<void> => {
    deps.setError(null);
    setAuthMode('checking');
    try {
      if (!(await getSessionState())) {
        setAuthMode('unauthenticated');
        return;
      }
      setAuthMode('authenticated');
      await deps.onAuthenticated();
    } catch (error) {
      setAuthMode('unauthenticated');
      deps.setError(readJsError(error, 'failed to initialize session'));
    }
  };

const createSubmitLogin =
  (
    deps: Readonly<AuthControllerDeps>,
    readAccessCode: () => string,
    runAuthenticatedLogin: (normalizedCode: string) => Promise<void>,
  ): ((event: PreventDefaultEvent) => Promise<void>) =>
  async (event: PreventDefaultEvent): Promise<void> => {
    event.preventDefault();
    const normalizedCode = normalizeCode(readAccessCode());
    if (normalizedCode === null) {
      deps.setError(REQUIRED_CODE_ERROR);
      return;
    }
    await runAuthenticatedLogin(normalizedCode);
  };

const createSignOut =
  (
    deps: Readonly<AuthControllerDeps>,
    setAuthMode: (mode: AuthMode) => void,
    setBusy: (value: boolean) => void,
  ): (() => Promise<void>) =>
  async (): Promise<void> => {
    setBusy(true);
    deps.setError(null);
    try {
      await logout();
      deps.channelsController.resetState();
      setAuthMode('unauthenticated');
      navigate('/twitch');
    } catch (error) {
      deps.setError(readJsError(error, 'logout failed'));
    } finally {
      setBusy(false);
    }
  };

const createRunAuthenticatedLogin =
  (
    deps: Readonly<AuthControllerDeps>,
    setBusy: (value: boolean) => void,
    setLoggedInState: () => void,
  ): ((normalizedCode: string) => Promise<void>) =>
  async (normalizedCode: string): Promise<void> => {
    setBusy(true);
    deps.setError(null);
    try {
      await login(normalizedCode);
      setLoggedInState();
      await deps.onAuthenticated();
    } catch (error) {
      deps.setError(readJsError(error, 'login failed'));
    } finally {
      setBusy(false);
    }
  };

export const createAuthController = (deps: Readonly<AuthControllerDeps>): AuthController => {
  let authMode = $state<AuthMode>('checking');
  let isBusy = $state(false);
  let accessCode = $state('');

  const setAuthMode = (mode: AuthMode): void => {
    authMode = mode;
  };

  const setAccessCode = (value: string): void => {
    accessCode = value;
  };

  const setBusy = (value: boolean): void => {
    isBusy = value;
  };

  const runAuthenticatedLogin = createRunAuthenticatedLogin(deps, setBusy, () => {
    setAccessCode('');
    setAuthMode('authenticated');
  });

  const initialize = createInitialize(deps, setAuthMode);
  const submitLogin = createSubmitLogin(deps, () => accessCode, runAuthenticatedLogin);

  return {
    get accessCode() {
      return accessCode;
    },
    get authMode() {
      return authMode;
    },
    initialize,
    get isBusy() {
      return isBusy;
    },
    setAccessCode,
    signOut: createSignOut(deps, setAuthMode, setBusy),
    submitLogin,
  };
};
