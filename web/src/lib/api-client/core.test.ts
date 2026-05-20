import { describe, expect, it, vi } from 'vitest';

import { isObject, readApiError, request, safeJson } from './core.js';

// Test constants
const TEST_NUMBER_123 = 123;
const TEST_NUMBER_1 = 1;
const TEST_NUMBER_2 = 2;
const TEST_NUMBER_3 = 3;
const TEST_BIGINT = 123;
const VALUE_A = 1;
const TEST_SYMBOL_LABEL = 'test';
const NOOP_WITH_VALUE = (): number => TEST_NUMBER_1;
const emptyFn = (): number => TEST_NUMBER_2;

describe('api-client/core isObject', () => {
  it('returns true for plain objects', () => {
    expect(isObject({})).toBe(true);
    expect(isObject({ valueA: VALUE_A })).toBe(true);
    expect(isObject({ nested: { value: 'test' } })).toBe(true);
  });

  it('returns false for null', () => {
    expect(isObject(null)).toBe(false);
  });

  it('returns true for arrays (arrays are objects in JS)', () => {
    // Note: arrays are technically objects in JavaScript
    expect(isObject([])).toBe(true);
    expect(isObject([TEST_NUMBER_1, TEST_NUMBER_2, TEST_NUMBER_3])).toBe(true);
  });

  it('returns false for primitives', () => {
    expect(isObject('string')).toBe(false);
    expect(isObject(TEST_NUMBER_123)).toBe(false);
    expect(isObject(true)).toBe(false);
    expect(isObject(Symbol(TEST_SYMBOL_LABEL))).toBe(false);
    expect(isObject(BigInt(TEST_BIGINT))).toBe(false);
  });

  it('returns false for functions', () => {
    expect(isObject(NOOP_WITH_VALUE)).toBe(false);
    expect(isObject(emptyFn)).toBe(false);
  });
});

describe('api-client/core safeJson', () => {
  it('returns parsed JSON on success', async () => {
    const data = { number: 42, test: 'value' };
    const response = Response.json(data);

    const result = await safeJson(response);
    expect(result).toEqual(data);
  });

  it('returns undefined on parse error', async () => {
    const response = new Response('not valid json');

    const result = await safeJson(response);
    expect(result).toBeUndefined();
  });

  it('handles empty string response', async () => {
    const response = new Response('');

    const result = await safeJson(response);
    expect(result).toBeUndefined();
  });
});

describe('api-client/core readApiError', () => {
  const READ_ERROR_TEST_VALUE_123 = 123;

  it('extracts error message from valid error object', () => {
    const payload = { error: 'Something went wrong' };
    expect(readApiError(payload)).toBe('Something went wrong');
  });

  it('returns default message for non-object payload', () => {
    expect(readApiError('string')).toBe('request failed');
    expect(readApiError(READ_ERROR_TEST_VALUE_123)).toBe('request failed');
    expect(readApiError(null)).toBe('request failed');
    expect(readApiError(null)).toBe('request failed');
    expect(readApiError([])).toBe('request failed');
  });

  it('returns default message when error property is missing', () => {
    expect(readApiError({})).toBe('request failed');
    expect(readApiError({ message: 'test' })).toBe('request failed');
    expect(readApiError({ success: true })).toBe('request failed');
  });

  it('returns default message when error property is not a string', () => {
    expect(readApiError({ error: READ_ERROR_TEST_VALUE_123 })).toBe('request failed');
    expect(readApiError({ error: null })).toBe('request failed');
    expect(readApiError({ error: true })).toBe('request failed');
    expect(readApiError({ error: {} })).toBe('request failed');
    expect(readApiError({ error: [] })).toBe('request failed');
  });

  it('handles complex error objects', () => {
    const payload = {
      details: { field: 'name', reason: 'required' },
      error: 'Validation failed',
    };
    expect(readApiError(payload)).toBe('Validation failed');
  });
});

describe('api-client/core request', () => {
  it('calls fetch with correct parameters', async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response('{}'));
    globalThis.fetch = mockFetch;

    await request('/api/test');

    expect(mockFetch).toHaveBeenCalledWith('/api/test', {
      credentials: 'same-origin',
    });
  });
});

describe('api-client/core request options', () => {
  it('merges custom init options', async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response('{}'));
    globalThis.fetch = mockFetch;

    const init: RequestInit = {
      body: JSON.stringify({ test: true }),
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    };

    await request('/api/test', init);

    expect(mockFetch).toHaveBeenCalledWith('/api/test', {
      body: JSON.stringify({ test: true }),
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json' },
      method: 'POST',
    });
  });

  it('preserves credentials when init is provided', async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response('{}'));
    globalThis.fetch = mockFetch;

    await request('/api/test', { method: 'GET' });

    expect(mockFetch).toHaveBeenCalledWith('/api/test', {
      credentials: 'same-origin',
      method: 'GET',
    });
  });
});

describe('api-client/core request response', () => {
  it('returns the fetch response', async () => {
    const expectedResponse = new Response('{}', { status: 200 });
    globalThis.fetch = vi.fn().mockResolvedValue(expectedResponse);

    const result = await request('/api/test');

    expect(result).toBe(expectedResponse);
  });
});
