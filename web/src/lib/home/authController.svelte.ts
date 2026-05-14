import { getSessionState, login, logout } from "$lib/api";
import { readJsError } from "$lib/home/errors";

export type AuthMode = "checking" | "authenticated" | "unauthenticated";

export interface AuthControllerDeps {
  setError: (message: string | null) => void;
  onAuthenticated: () => Promise<void>;
}

export interface AuthController {
  authMode: AuthMode;
  isBusy: boolean;
  accessCode: string;
  initialize: () => Promise<void>;
  submitLogin: (event: SubmitEvent) => Promise<void>;
  signOut: () => Promise<void>;
  setAccessCode: (value: string) => void;
}

export function createAuthController(deps: AuthControllerDeps): AuthController {
  let authMode = $state<AuthMode>("checking");
  let isBusy = $state(false);
  let accessCode = $state("");

  const { setError, onAuthenticated } = deps;

  async function initialize(): Promise<void> {
    setError(null);
    authMode = "checking";

    try {
      const authenticated = await getSessionState();
      authMode = authenticated ? "authenticated" : "unauthenticated";
      if (authenticated) {
        await onAuthenticated();
      }
    } catch (err) {
      authMode = "unauthenticated";
      setError(readJsError(err, "failed to initialize session"));
    }
  }

  async function submitLogin(event: SubmitEvent): Promise<void> {
    event.preventDefault();

    const normalized = accessCode.trim();
    if (!normalized) {
      setError("access code is required");
      return;
    }

    isBusy = true;
    setError(null);

    try {
      await login(normalized);
      accessCode = "";
      authMode = "authenticated";
      await onAuthenticated();
    } catch (err) {
      setError(readJsError(err, "login failed"));
    } finally {
      isBusy = false;
    }
  }

  async function signOut(): Promise<void> {
    isBusy = true;
    setError(null);

    try {
      await logout();
      authMode = "unauthenticated";
    } catch (err) {
      setError(readJsError(err, "logout failed"));
    } finally {
      isBusy = false;
    }
  }

  function setAccessCode(value: string): void {
    accessCode = value;
  }

  return {
    get authMode() {
      return authMode;
    },
    get isBusy() {
      return isBusy;
    },
    get accessCode() {
      return accessCode;
    },
    initialize,
    submitLogin,
    signOut,
    setAccessCode,
  };
}
