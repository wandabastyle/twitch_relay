import { getSessionState, login, logout } from '$lib/api-client';
import { readJsError } from '$lib/home/errors';

export type AuthMode = 'authenticated' | 'checking' | 'unauthenticated';

export interface AuthControllerDeps {
  onAuthenticated: () => Promise<void>;
  setError: (message: string | null) => void;
}

export interface AuthController {
  accessCode: string;
  authMode: AuthMode;
  initialize: () => Promise<void>;
  isBusy: boolean;
  setAccessCode: (value: string) => void;
  signOut: () => Promise<void>;
  submitLogin: (event: SubmitEvent) => Promise<void>;
}

export const createAuthController = (deps: Readonly<AuthControllerDeps>): AuthController => {
  let authMode = $state<AuthMode>('checking');
  let isBusy = $state(false);
  let accessCode = $state('');

  const { onAuthenticated, setError } = deps;

  const initialize = async (): Promise<void> => {
    setError(null);
    authMode = 'checking';

    try {
      const authenticated = await getSessionState();
      if (authenticated) {
        authMode = 'authenticated';
        await onAuthenticated();
      } else {
        authMode = 'unauthenticated';
      }
    } catch (error) {
      authMode = 'unauthenticated';
      setError(readJsError(error, 'failed to initialize session'));
    }
  };

  const submitLogin = async (event: Readonly<SubmitEvent>): Promise<void> => {
    event.preventDefault();

    const normalized = accessCode.trim();
    if (!normalized) {
      setError('access code is required');
      return;
    }

    isBusy = true;
    setError(null);

    try {
      await login(normalized);
      accessCode = '';
      authMode = 'authenticated';
      await onAuthenticated();
    } catch (error) {
      setError(readJsError(error, 'login failed'));
    } finally {
      isBusy = false;
    }
  };

  const signOut = async (): Promise<void> => {
    isBusy = true;
    setError(null);

    try {
      await logout();
      authMode = 'unauthenticated';
    } catch (error) {
      setError(readJsError(error, 'logout failed'));
    } finally {
      isBusy = false;
    }
  };

  const setAccessCode = (value: string): void => {
    accessCode = value;
  };

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
    signOut,
    submitLogin,
  };
};
