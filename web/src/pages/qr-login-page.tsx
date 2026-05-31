import { useCallback, useState, type ReactElement } from 'react';
import { login } from '../api-client';

interface QrLoginPageProps {
  token: string;
}

const REQUIRED_ERROR = 'access code is required';
const LOGIN_FAILED = 'login failed';
const MIN_MESSAGE_LENGTH = 0;

const readMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message.trim().length > MIN_MESSAGE_LENGTH) {
    return error.message;
  }
  return fallback;
};

export const QrLoginPage = ({ token }: QrLoginPageProps): ReactElement => {
  const [accessCode, setAccessCode] = useState('');
  const [isBusy, setIsBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | undefined>();
  const [success, setSuccess] = useState(false);

  const handleSubmit = useCallback(
    (event: React.SyntheticEvent<HTMLFormElement>): void => {
      event.preventDefault();

      const normalized = accessCode.trim();
      if (normalized === '') {
        setErrorMessage(REQUIRED_ERROR);
        return;
      }

      setIsBusy(true);
      setErrorMessage(undefined);

      void (async (): Promise<void> => {
        try {
          await login(normalized, token);
          setSuccess(true);
        } catch (error) {
          setErrorMessage(readMessage(error, LOGIN_FAILED));
        } finally {
          setIsBusy(false);
        }
      })();
    },
    [accessCode, token],
  );

  const handleChange = useCallback((event: React.ChangeEvent<HTMLInputElement>): void => {
    setAccessCode(event.target.value);
  }, []);

  return (
    <main className="ui-page-shell ui-page-shell--centered">
      <section className="ui-page-panel ui-page-panel--narrow">
        <header className="ui-panel-header--centered">
          <p className="ui-page-eyebrow">QR Login</p>
          <h1 className="ui-page-title">Twitch Relay</h1>
        </header>

        {errorMessage !== undefined && (
          <p className="ui-error" role="alert">
            {errorMessage}
          </p>
        )}

        {success ? (
          <div className="ui-success-message">
            <p className="ui-success-text">Console logged in successfully!</p>
            <p className="ui-success-subtext">You can close this window.</p>
          </div>
        ) : (
          <form className="ui-form" onSubmit={handleSubmit}>
            <label className="ui-label" htmlFor="access-code">
              Access code
            </label>
            <input
              id="access-code"
              type="password"
              value={accessCode}
              onChange={handleChange}
              placeholder="Enter shared access code"
              autoComplete="current-password"
              disabled={isBusy}
              className="qr-login-input"
            />
            <button className="ui-button-primary" type="submit" disabled={isBusy}>
              {isBusy ? 'Signing in...' : 'Sign in'}
            </button>
          </form>
        )}
      </section>
    </main>
  );
}
