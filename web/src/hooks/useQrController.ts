import QRCode from 'qrcode';
import { useCallback, useRef, useState } from 'react';
import { claimQrSession, createQrSession, getQrStatus } from '../api-client';
import { readJsError } from './errors';

const QR_POLL_INTERVAL_MS = 3000;

const QR_CODE_OPTIONS = {
  color: {
    dark: '#c8d3f5',
    light: '#2f334d',
  },
  margin: 2,
  width: 200,
} as const;

interface QrControllerDeps {
  onQrAuthenticated: () => void;
  setError: (message: string | null) => void;
}

export interface QrController {
  cleanup: () => void;
  loginMode: 'code' | 'qr';
  qrDataUrl: string | undefined;
  qrToken: string | undefined;
  switchToCodeMode: () => void;
  switchToQrMode: () => Promise<void>;
}

const buildQrDataUrl = async (token: string): Promise<string> => {
  const qrUrl = `${window.location.origin}/qr-login/${encodeURIComponent(token)}`;
  const dataUrl = await QRCode.toDataURL(qrUrl, QR_CODE_OPTIONS);
  return dataUrl;
};

export function useQrController(deps: QrControllerDeps): QrController {
  const [loginMode, setLoginMode] = useState<'code' | 'qr'>('code');
  const [qrToken, setQrToken] = useState<string | undefined>();
  const [qrDataUrl, setQrDataUrl] = useState<string | undefined>();
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const clearQrPolling = useCallback(() => {
    if (pollIntervalRef.current !== null) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  const claimSession = useCallback(async () => {
    if (qrToken === undefined) {
      return;
    }

    try {
      await claimQrSession(qrToken);
      deps.onQrAuthenticated();
    } catch (error) {
      deps.setError(readJsError(error, 'failed to claim session'));
      switchToCodeMode();
    }
  }, [qrToken, deps]);

  const pollQrStatus = useCallback(async () => {
    if (qrToken === undefined) {
      return;
    }

    try {
      const status = await getQrStatus(qrToken);
      if (status.status !== 'authenticated') {
        return;
      }

      clearQrPolling();
      await claimSession();
    } catch {
      // Ignore polling errors, session might just not be ready yet
    }
  }, [qrToken, clearQrPolling, claimSession]);

  const startQrPolling = useCallback(() => {
    clearQrPolling();
    if (qrToken === undefined) {
      return;
    }

    pollIntervalRef.current = setInterval(() => {
      void pollQrStatus();
    }, QR_POLL_INTERVAL_MS);
  }, [qrToken, clearQrPolling, pollQrStatus]);

  const generateQrCode = useCallback(async () => {
    try {
      const session = await createQrSession();
      setQrToken(session.token);
      const dataUrl = await buildQrDataUrl(session.token);
      setQrDataUrl(dataUrl);
      startQrPolling();
    } catch (error) {
      deps.setError(readJsError(error, 'failed to generate QR code'));
      switchToCodeMode();
    }
  }, [deps, startQrPolling]);

  const switchToCodeMode = useCallback(() => {
    setLoginMode('code');
    deps.setError(null);
    clearQrPolling();
    setQrToken(undefined);
    setQrDataUrl(undefined);
  }, [deps, clearQrPolling]);

  const switchToQrMode = useCallback(async () => {
    setLoginMode('qr');
    deps.setError(null);
    await generateQrCode();
  }, [deps, generateQrCode]);

  const cleanup = useCallback(() => {
    clearQrPolling();
  }, [clearQrPolling]);

  return {
    cleanup,
    loginMode,
    qrDataUrl,
    qrToken,
    switchToCodeMode,
    switchToQrMode,
  };
}
