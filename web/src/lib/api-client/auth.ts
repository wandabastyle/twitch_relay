import { isObject, readApiError, request, safeJson } from './core.js';
import type { QrSessionResponse, QrStatusResponse } from './types.js';

const AUTHENTICATED_STATUS = 'authenticated';
const PENDING_STATUS = 'pending';

const createLoginBody = (accessCode: string, qrToken?: string): Record<string, string> => {
  const body: Record<string, string> = { access_code: accessCode };
  if (qrToken !== undefined && qrToken !== '') {
    body.qr_token = qrToken;
  }
  return body;
};

const parseQrStatusPayload = (payload: unknown, errorMessage: string): QrStatusResponse => {
  if (!isObject(payload) || typeof payload.status !== 'string') {
    throw new Error(errorMessage);
  }
  return {
    status: payload.status === AUTHENTICATED_STATUS ? AUTHENTICATED_STATUS : PENDING_STATUS,
  };
};

const throwLoginError = (payload: unknown): never => {
  if (isObject(payload) && typeof payload.error === 'string' && payload.error !== '') {
    throw new Error(payload.error);
  }
  throw new Error('login failed');
};

export const getSessionState = async (): Promise<boolean> => {
  const response = await request('/auth/session');
  if (!response.ok) {
    throw new Error(`session request failed (${String(response.status)})`);
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.authenticated !== 'boolean') {
    throw new Error('session response payload is invalid');
  }

  return payload.authenticated;
};

export const login = async (accessCode: string, qrToken?: string): Promise<void> => {
  const body = createLoginBody(accessCode, qrToken);
  const response = await request('/auth/login', {
    body: JSON.stringify(body),
    headers: {
      'content-type': 'application/json',
    },
    method: 'POST',
  });

  if (response.ok) {
    return;
  }
  throwLoginError(await safeJson(response));
};

export const createQrSession = async (): Promise<QrSessionResponse> => {
  const response = await request('/auth/qr/create');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    typeof payload.token !== 'string' ||
    typeof payload.expires_at !== 'number'
  ) {
    throw new Error('qr session response payload is invalid');
  }

  return {
    expires_at: payload.expires_at,
    token: payload.token,
  };
};

export const getQrStatus = async (token: string): Promise<QrStatusResponse> => {
  const response = await request(`/auth/qr/status/${encodeURIComponent(token)}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  return parseQrStatusPayload(await safeJson(response), 'qr status response payload is invalid');
};

export const claimQrSession = async (token: string): Promise<QrStatusResponse> => {
  const response = await request(`/auth/qr/claim/${encodeURIComponent(token)}`, {
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  return parseQrStatusPayload(await safeJson(response), 'qr claim response payload is invalid');
};

export const logout = async (): Promise<void> => {
  await request('/auth/logout', { method: 'POST' });
};
