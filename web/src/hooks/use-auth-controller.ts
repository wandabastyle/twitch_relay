import { useCallback, useState } from 'react';
import { getSessionState, login, logout } from '../api-client';
import { navigate } from '../router';
import type { ChannelsController } from './channels-controller';
import { readJsError } from './errors';

export type AuthMode = 'authenticated' | 'checking' | 'unauthenticated';

export interface AuthControllerDeps {
  channelsController: ChannelsController;
  onAuthenticated: () => Promise<void>;
  setError: (message: string | null) => void;
}

interface PreventDefaultEvent {
  preventDefault: () => void;
}

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

export const useAuthController = (deps: AuthControllerDeps): AuthController => {
  const [authMode, setAuthMode] = useState<AuthMode>('checking');
  const [isBusy, setIsBusy] = useState(false);
  const [accessCode, setAccessCodeState] = useState('');

  const setAccessCode = useCallback((value: string) => {
    setAccessCodeState(value);
  }, []);

  const setBusy = useCallback((value: boolean) => {
    setIsBusy(value);
  }, []);

  const setAuthModeCallback = useCallback((mode: AuthMode) => {
    setAuthMode(mode);
  }, []);

  const initialize = useCallback(async (): Promise<void> => {
    deps.setError(null);
    setAuthModeCallback('checking');
    try {
      if (!(await getSessionState())) {
        setAuthModeCallback('unauthenticated');
        return;
      }
      setAuthModeCallback('authenticated');
      await deps.onAuthenticated();
    } catch (error) {
      setAuthModeCallback('unauthenticated');
      deps.setError(readJsError(error, 'failed to initialize session'));
    }
  }, [deps, setAuthModeCallback]);

  const signOut = useCallback(async (): Promise<void> => {
    setBusy(true);
    deps.setError(null);
    try {
      await logout();
      deps.channelsController.resetState();
      setAuthModeCallback('unauthenticated');
      navigate('/twitch');
    } catch (error) {
      deps.setError(readJsError(error, 'logout failed'));
    } finally {
      setBusy(false);
    }
  }, [deps, setAuthModeCallback, setBusy]);

  const runAuthenticatedLogin = useCallback(
    async (normalizedCode: string): Promise<void> => {
      setBusy(true);
      deps.setError(null);
      try {
        await login(normalizedCode);
        setAccessCode('');
        setAuthModeCallback('authenticated');
        await deps.onAuthenticated();
      } catch (error) {
        deps.setError(readJsError(error, 'login failed'));
      } finally {
        setBusy(false);
      }
    },
    [deps, setAuthModeCallback, setBusy],
  );

  const submitLogin = useCallback(
    async (event: PreventDefaultEvent): Promise<void> => {
      event.preventDefault();
      const normalizedCode = normalizeCode(accessCode);
      if (normalizedCode === null) {
        deps.setError(REQUIRED_CODE_ERROR);
        return;
      }
      await runAuthenticatedLogin(normalizedCode);
    },
    [accessCode, deps, runAuthenticatedLogin],
  );

  return {
    accessCode,
    authMode,
    initialize,
    isBusy,
    setAccessCode,
    signOut,
    submitLogin,
  };
};
