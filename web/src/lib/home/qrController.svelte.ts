import { claimQrSession, createQrSession, getQrStatus } from '$lib/api-client';
import { readJsError } from '$lib/home/errors';
import QRCode from 'qrcode';

const QR_POLL_INTERVAL_MS = 3000;

const QR_CODE_OPTIONS = {
  color: {
    dark: '#c8d3f5',
    light: '#2f334d',
  },
  margin: 2,
  width: 200,
} as const;

export interface QrControllerDeps {
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

export const createQrController = (deps: Readonly<QrControllerDeps>): QrController => {
  let loginMode = $state<'code' | 'qr'>('code');
  let qrToken = $state<string>();
  let qrDataUrl = $state<string>();
  let qrPollInterval: ReturnType<typeof setInterval> | undefined = undefined;

  const { onQrAuthenticated, setError } = deps;

  const switchToQrMode = async (): Promise<void> => {
    loginMode = 'qr';
    setError(null);
    await generateQrCode();
  };

  const switchToCodeMode = (): void => {
    loginMode = 'code';
    setError(null);
    cleanup();
    qrToken = undefined;
    qrDataUrl = undefined;
  };

  const cleanup = (): void => {
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
      qrPollInterval = undefined;
    }
  };

  const generateQrCode = async (): Promise<void> => {
    try {
      const session = await createQrSession();
      qrToken = session.token;

      const qrUrl = `${globalThis.window.location.origin}/qr-login/${encodeURIComponent(session.token)}`;
      qrDataUrl = await QRCode.toDataURL(qrUrl, QR_CODE_OPTIONS);

      startQrPolling();
    } catch (error) {
      setError(readJsError(error, 'failed to generate QR code'));
      loginMode = 'code';
    }
  };

  const startQrPolling = (): void => {
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
    }

    if (qrToken === undefined) {
      return;
    }

    qrPollInterval = setInterval(() => {
      void (async (): Promise<void> => {
        if (qrToken === undefined) {
          return;
        }

        try {
          const status = await getQrStatus(qrToken);
          if (status.status === 'authenticated') {
            cleanup();
            try {
              await claimQrSession(qrToken);
              onQrAuthenticated();
            } catch (error) {
              setError(readJsError(error, 'failed to claim session'));
              switchToCodeMode();
            }
          }
        } catch {
          // Ignore polling errors, session might just not be ready yet
        }
      })();
    }, QR_POLL_INTERVAL_MS);
  };

  return {
    cleanup,
    get loginMode() {
      return loginMode;
    },
    get qrDataUrl() {
      return qrDataUrl;
    },
    get qrToken() {
      return qrToken;
    },
    switchToCodeMode,
    switchToQrMode,
  };
};
