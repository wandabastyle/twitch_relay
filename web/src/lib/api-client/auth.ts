import { isObject, readApiError, request, safeJson } from './core.js';
import type { QrSessionResponse, QrStatusResponse } from './types.js';

const AUTHENTICATED_STATUS = 'authenticated';
const PENDING_STATUS = 'pending';

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
  const body: Record<string, string> = { access_code: accessCode };
  if (qrToken !== undefined && qrToken !== '') {
    body.qr_token = qrToken;
  }

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

  const payload = await safeJson(response);
  if (!isObject(payload) && payload !== undefined) {
    throw new Error('login failed');
  }
  if (isObject(payload) && typeof payload.error === 'string' && payload.error !== '') {
    throw new Error(payload.error);
  }
  throw new Error('login failed');
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

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.status !== 'string') {
    throw new Error('qr status response payload is invalid');
  }

  let status: QrStatusResponse['status'] = PENDING_STATUS;
  if (payload.status === AUTHENTICATED_STATUS) {
    status = AUTHENTICATED_STATUS;
  }

  return { status };
};

export const claimQrSession = async (token: string): Promise<QrStatusResponse> => {
  const response = await request(`/auth/qr/claim/${encodeURIComponent(token)}`, {
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.status !== 'string') {
    throw new Error('qr claim response payload is invalid');
  }

  let status: QrStatusResponse['status'] = PENDING_STATUS;
  if (payload.status === AUTHENTICATED_STATUS) {
    status = AUTHENTICATED_STATUS;
  }

  return { status };
};

export const logout = async (): Promise<void> => {
  await request('/auth/logout', { method: 'POST' });
};
