import QRCode from "qrcode";
import { createQrSession, getQrStatus, claimQrSession } from "$lib/api";
import { readMessage } from "$lib/home/errors";
import { QR_POLL_INTERVAL_MS, QR_CODE_OPTIONS } from "$lib/home/qr";

export interface QrControllerDeps {
  setError: (message: string | null) => void;
  onQrAuthenticated: () => void;
}

export interface QrController {
  loginMode: "code" | "qr";
  qrToken: string | null;
  qrDataUrl: string | null;
  switchToQrMode: () => Promise<void>;
  switchToCodeMode: () => void;
  cleanup: () => void;
}

export function createQrController(deps: QrControllerDeps): QrController {
  let loginMode = $state<"code" | "qr">("code");
  let qrToken = $state<string | null>(null);
  let qrDataUrl = $state<string | null>(null);
  let qrPollInterval: ReturnType<typeof setInterval> | null = null;

  const { setError, onQrAuthenticated } = deps;

  async function switchToQrMode(): Promise<void> {
    loginMode = "qr";
    setError(null);
    await generateQrCode();
  }

  function switchToCodeMode(): void {
    loginMode = "code";
    setError(null);
    cleanup();
    qrToken = null;
    qrDataUrl = null;
  }

  function cleanup(): void {
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
      qrPollInterval = null;
    }
  }

  async function generateQrCode(): Promise<void> {
    try {
      const session = await createQrSession();
      qrToken = session.token;

      const qrUrl = `${window.location.origin}/qr-login/${encodeURIComponent(session.token)}`;
      qrDataUrl = await QRCode.toDataURL(qrUrl, QR_CODE_OPTIONS);

      startQrPolling();
    } catch (err) {
      setError(readMessage(err, "failed to generate QR code"));
      loginMode = "code";
    }
  }

  function startQrPolling(): void {
    if (qrPollInterval) {
      clearInterval(qrPollInterval);
    }

    if (!qrToken) return;

    qrPollInterval = setInterval(async () => {
      if (!qrToken) return;

      try {
        const status = await getQrStatus(qrToken);
        if (status.status === "authenticated") {
          cleanup();
          try {
            await claimQrSession(qrToken);
            onQrAuthenticated();
          } catch (err) {
            setError(readMessage(err, "failed to claim session"));
            switchToCodeMode();
          }
        }
      } catch {
        // Ignore polling errors, session might just not be ready yet
      }
    }, QR_POLL_INTERVAL_MS);
  }

  return {
    get loginMode() {
      return loginMode;
    },
    get qrToken() {
      return qrToken;
    },
    get qrDataUrl() {
      return qrDataUrl;
    },
    switchToQrMode,
    switchToCodeMode,
    cleanup,
  };
}
