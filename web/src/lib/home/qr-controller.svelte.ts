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

interface QrRuntime {
  onQrAuthenticated: () => void;
  setError: (message: string | null) => void;
  getQrToken: () => string | undefined;
  setQrToken: (value: string | undefined) => void;
  setQrDataUrl: (value: string | undefined) => void;
  setLoginMode: (value: 'code' | 'qr') => void;
  clearQrPolling: () => void;
  setQrPollInterval: (value: Readonly<ReturnType<typeof setInterval>> | undefined) => void;
}

const buildQrDataUrl = async (token: string): Promise<string> => {
  const qrUrl = `${globalThis.window.location.origin}/qr-login/${encodeURIComponent(token)}`;
  const dataUrl = await QRCode.toDataURL(qrUrl, QR_CODE_OPTIONS);
  return dataUrl;
};

const createSwitchToCodeMode =
  (runtime: Readonly<QrRuntime>, cleanup: () => void): (() => void) =>
  (): void => {
    runtime.setLoginMode('code');
    runtime.setError(null);
    cleanup();
    runtime.setQrToken(undefined);
    runtime.setQrDataUrl(undefined);
  };

const createClaimSession =
  (runtime: Readonly<QrRuntime>, switchToCodeMode: () => void): (() => Promise<void>) =>
  async (): Promise<void> => {
    const token = runtime.getQrToken();
    if (token === undefined) {
      return;
    }

    try {
      await claimQrSession(token);
      runtime.onQrAuthenticated();
    } catch (error) {
      runtime.setError(readJsError(error, 'failed to claim session'));
      switchToCodeMode();
    }
  };

const createPollQrStatus =
  (
    runtime: Readonly<QrRuntime>,
    cleanup: () => void,
    claimSession: () => Promise<void>,
  ): (() => Promise<void>) =>
  async (): Promise<void> => {
    const token = runtime.getQrToken();
    if (token === undefined) {
      return;
    }

    try {
      const status = await getQrStatus(token);
      if (status.status !== 'authenticated') {
        return;
      }

      cleanup();
      await claimSession();
    } catch {
      // Ignore polling errors, session might just not be ready yet
    }
  };

const createStartQrPolling =
  (runtime: Readonly<QrRuntime>, pollQrStatus: () => Promise<void>): (() => void) =>
  (): void => {
    runtime.clearQrPolling();
    if (runtime.getQrToken() === undefined) {
      return;
    }

    runtime.setQrPollInterval(
      setInterval(() => {
        void pollQrStatus();
      }, QR_POLL_INTERVAL_MS),
    );
  };

const createGenerateQrCode =
  (
    runtime: Readonly<QrRuntime>,
    startQrPolling: () => void,
    switchToCodeMode: () => void,
  ): (() => Promise<void>) =>
  async (): Promise<void> => {
    try {
      const session = await createQrSession();
      runtime.setQrToken(session.token);
      runtime.setQrDataUrl(await buildQrDataUrl(session.token));
      startQrPolling();
    } catch (error) {
      runtime.setError(readJsError(error, 'failed to generate QR code'));
      switchToCodeMode();
    }
  };

const createQrActions = (
  runtime: Readonly<QrRuntime>,
): Readonly<Pick<QrController, 'cleanup' | 'switchToCodeMode' | 'switchToQrMode'>> => {
  const cleanup = (): void => {
    runtime.clearQrPolling();
  };
  const switchToCodeMode = createSwitchToCodeMode(runtime, cleanup);
  const claimSession = createClaimSession(runtime, switchToCodeMode);
  const pollQrStatus = createPollQrStatus(runtime, cleanup, claimSession);
  const startQrPolling = createStartQrPolling(runtime, pollQrStatus);
  const generateQrCode = createGenerateQrCode(runtime, startQrPolling, switchToCodeMode);

  const switchToQrMode = async (): Promise<void> => {
    runtime.setLoginMode('qr');
    runtime.setError(null);
    await generateQrCode();
  };

  return { cleanup, switchToCodeMode, switchToQrMode };
};

export const createQrController = (deps: Readonly<QrControllerDeps>): QrController => {
  let loginMode = $state<'code' | 'qr'>('code');
  let qrToken = $state<string>();
  let qrDataUrl = $state<string>();
  let qrPollInterval: ReturnType<typeof setInterval> | undefined = undefined;

  const clearQrPolling = (): void => {
    if (!qrPollInterval) {
      return;
    }

    clearInterval(qrPollInterval);
    qrPollInterval = undefined;
  };
  const actions = createQrActions({
    clearQrPolling,
    getQrToken: () => qrToken,
    onQrAuthenticated: deps.onQrAuthenticated,
    setError: deps.setError,
    setLoginMode: (value) => {
      loginMode = value;
    },
    setQrDataUrl: (value) => {
      qrDataUrl = value;
    },
    setQrPollInterval: (value: Readonly<ReturnType<typeof setInterval>> | undefined) => {
      qrPollInterval = value;
    },
    setQrToken: (value) => {
      qrToken = value;
    },
  });

  return {
    cleanup: actions.cleanup,
    get loginMode() {
      return loginMode;
    },
    get qrDataUrl() {
      return qrDataUrl;
    },
    get qrToken() {
      return qrToken;
    },
    switchToCodeMode: actions.switchToCodeMode,
    switchToQrMode: actions.switchToQrMode,
  };
};
