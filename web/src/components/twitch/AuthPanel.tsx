import type { FormEvent, ReactElement } from 'react';

interface AuthPanelProps {
  accessCode: string;
  isBusy: boolean;
  loginMode: 'code' | 'qr';
  onSubmitLogin: (event: FormEvent) => void;
  onSwitchToCode: () => void;
  onSwitchToQr: () => void;
  onUpdateAccessCode: (value: string) => void;
  qrDataUrl: string | undefined;
}

export const AuthPanel = ({
  accessCode,
  isBusy,
  loginMode,
  onSubmitLogin,
  onSwitchToCode,
  onSwitchToQr,
  onUpdateAccessCode,
  qrDataUrl,
}: AuthPanelProps): ReactElement => {
  if (loginMode === 'code') {
    return (
      <form className="login-form" onSubmit={onSubmitLogin}>
        <label htmlFor="access-code">Access code</label>
        <input
          id="access-code"
          className="ui-input"
          type="password"
          value={accessCode}
          onChange={(e) => onUpdateAccessCode(e.currentTarget.value)}
          placeholder="Enter shared access code"
          autoComplete="current-password"
        />
        <button type="submit" disabled={isBusy}>
          {isBusy ? 'Signing in...' : 'Sign in'}
        </button>
        <button type="button" onClick={onSwitchToQr}>
          Sign in with QR code
        </button>
      </form>
    );
  }

  return (
    <div className="qr-login">
      {qrDataUrl ? (
        <img src={qrDataUrl} alt="QR Code for login" className="qr-code" />
      ) : (
        <div className="qr-placeholder">Generating QR code...</div>
      )}
      <p className="qr-instructions">
        Scan with your phone
        <br />
        <span className="qr-expires">expires in 5 minutes</span>
      </p>
      <button type="button" onClick={onSwitchToCode}>
        Sign in with access code
      </button>
    </div>
  );
}
