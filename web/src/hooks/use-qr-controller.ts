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
  const qrUrl = `${globalThis.location.origin}/qr-login/${encodeURIComponent(token)}`;
  const dataUrl = await QRCode.toDataURL(qrUrl, QR_CODE_OPTIONS);
  return dataUrl;
};

export const useQrController = (deps: QrControllerDeps): QrController => {
  const [loginMode, setLoginMode] = useState<'code' | 'qr'>('code');
  const [qrToken, setQrToken] = useState<string | undefined>();
  const [qrDataUrl, setQrDataUrl] = useState<string | undefined>();
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  // Use ref for token to avoid stale closure in polling
  const tokenRef = useRef<string | null>(null);

  const clearQrPolling = useCallback(() => {
    if (pollIntervalRef.current !== null) {
      clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  const resetQrState = useCallback(() => {
    clearQrPolling();
    setQrToken(undefined);
    tokenRef.current = null;
    setQrDataUrl(undefined);
  }, [clearQrPolling]);

  // Update ref when token changes
  tokenRef.current = qrToken ?? null;
  // Avoid unused variable warning - tokenRef is used for polling
  void tokenRef.current;

  const claimSession = useCallback(
    async (token: string) => {
      try {
        await claimQrSession(token);
        deps.onQrAuthenticated();
      } catch (error) {
        deps.setError(readJsError(error, 'failed to claim session'));
        throw error;
      }
    },
    [deps],
  );

  const pollQrStatus = useCallback(async () => {
    const token = tokenRef.current;
    if (token === null) {
      return;
    }

    try {
      const status = await getQrStatus(token);
      if (status.status !== 'authenticated') {
        return;
      }

      clearQrPolling();
      await claimSession(token);
    } catch {
      // Ignore polling errors, session might just not be ready yet
    }
  }, [clearQrPolling, claimSession]);

  const startQrPolling = useCallback(() => {
    clearQrPolling();
    if (tokenRef.current === null) {
      return;
    }

    pollIntervalRef.current = setInterval(() => {
      void pollQrStatus();
    }, QR_POLL_INTERVAL_MS);
  }, [clearQrPolling, pollQrStatus]);

  const generateQrCode = useCallback(async () => {
    try {
      const session = await createQrSession();
      setQrToken(session.token);
      tokenRef.current = session.token;
      const dataUrl = await buildQrDataUrl(session.token);
      setQrDataUrl(dataUrl);
      startQrPolling();
    } catch (error) {
      deps.setError(readJsError(error, 'failed to generate QR code'));
      throw error;
    }
  }, [deps, startQrPolling]);

  const switchToCodeMode = useCallback(() => {
    setLoginMode('code');
    deps.setError(null);
    resetQrState();
  }, [deps, resetQrState]);

  const switchToQrMode = useCallback(async () => {
    setLoginMode('qr');
    deps.setError(null);
    try {
      await generateQrCode();
    } catch {
      // If QR generation fails, switch back to code mode
      switchToCodeMode();
    }
  }, [deps, generateQrCode, switchToCodeMode]);

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
};
