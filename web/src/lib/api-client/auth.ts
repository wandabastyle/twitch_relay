import { isObject, safeJson, readApiError, request } from './core';
import type { QrSessionResponse, QrStatusResponse } from './types';

export async function getSessionState(): Promise<boolean> {
  const response = await request('/auth/session');
  if (!response.ok) {
    throw new Error(`session request failed (${String(response.status)})`);
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.authenticated !== 'boolean') {
    throw new Error('session response payload is invalid');
  }

  return payload.authenticated;
}

export async function login(accessCode: string, qrToken?: string): Promise<void> {
  const body: Record<string, string> = { access_code: accessCode };
  if (qrToken) {
    body.qr_token = qrToken;
  }

  const response = await request('/auth/login', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  if (response.ok) {
    return;
  }

  const payload = (await safeJson(response)) as { error?: string };
  throw new Error(payload.error ?? 'login failed');
}

export async function createQrSession(): Promise<QrSessionResponse> {
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
    token: payload.token,
    expires_at: payload.expires_at,
  };
}

export async function getQrStatus(token: string): Promise<QrStatusResponse> {
  const response = await request(`/auth/qr/status/${encodeURIComponent(token)}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || typeof payload.status !== 'string') {
    throw new Error('qr status response payload is invalid');
  }

  return {
    status: payload.status === 'authenticated' ? 'authenticated' : 'pending',
  };
}

export async function claimQrSession(token: string): Promise<QrStatusResponse> {
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

  return {
    status: payload.status === 'authenticated' ? 'authenticated' : 'pending',
  };
}

export async function logout(): Promise<void> {
  await request('/auth/logout', { method: 'POST' });
}
