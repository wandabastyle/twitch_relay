import type {
  ActiveRecording,
  RecordingFileEntry,
  RecordingJobStartResponse,
  RecordingJobStatusResponse,
  RecordingRule,
  RecordingWatchProgress,
  RecordingsResponse,
} from './types.js';
import { isObject, readApiError, request, safeJson } from './core.js';

export const getRecordingRules = async (): Promise<RecordingRule[]> => {
  const response = await request('/api/recording-rules');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.rules)) {
    throw new Error('recording rules payload is invalid');
  }

  return payload.rules as RecordingRule[];
};

export const upsertRecordingRule = async (rule: {
  channel_login: string;
  enabled: boolean;
  keep_last_videos?: number | null;
  max_duration_minutes?: number | null;
  quality?: string;
  stop_when_offline?: boolean;
}): Promise<RecordingRule> => {
  const response = await request('/api/recording-rules', {
    body: JSON.stringify(rule),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  return (await safeJson(response)) as RecordingRule;
};

export const getRecordings = async (): Promise<RecordingsResponse> => {
  const response = await request('/api/recordings');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !Array.isArray(payload.active) ||
    !Array.isArray(payload.completed) ||
    !Array.isArray(payload.incomplete)
  ) {
    throw new Error('recordings payload is invalid');
  }

  return {
    active: payload.active as ActiveRecording[],
    completed: payload.completed as RecordingFileEntry[],
    incomplete: payload.incomplete as RecordingFileEntry[],
  };
};

export const startRecording = async (
  channel_login: string,
  quality?: string,
  stream_title?: string,
): Promise<void> => {
  const response = await request('/api/recordings/start', {
    body: JSON.stringify({ channel_login, quality, stream_title }),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
};

export const stopRecording = async (channel_login: string): Promise<void> => {
  const response = await request('/api/recordings/stop', {
    body: JSON.stringify({ channel_login }),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
};

export const deleteRecordingFile = async (payload: {
  bucket: 'completed' | 'incomplete';
  channel_login: string;
  filename: string;
}): Promise<void> => {
  const response = await request('/api/recordings/delete', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
};

export const pinRecordingFile = async (payload: {
  bucket: 'completed';
  channel_login: string;
  filename: string;
}): Promise<void> => {
  const response = await request('/api/recordings/pin', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
};

export const unpinRecordingFile = async (payload: {
  bucket: 'completed';
  channel_login: string;
  filename: string;
}): Promise<void> => {
  const response = await request('/api/recordings/unpin', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
};

export const getRecordingWatchProgress = async (
  channel_login: string,
  filename: string,
): Promise<RecordingWatchProgress> => {
  const params = new URLSearchParams({ channel_login, filename });
  const response = await request(`/api/recordings/progress?${params.toString()}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }
  return (await safeJson(response)) as RecordingWatchProgress;
};

export const saveRecordingWatchProgress = async (payload: {
  channel_login: string;
  completed?: boolean;
  duration_secs?: number;
  filename: string;
  position_secs: number;
}): Promise<RecordingWatchProgress> => {
  const response = await request('/api/recordings/progress', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'PUT',
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  return (await safeJson(response)) as RecordingWatchProgress;
};

export const mergeRecordingFiles = async (payload: {
  channel_login: string;
  filenames: string[];
}): Promise<RecordingJobStartResponse> => {
  const response = await request('/api/recordings/merge', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || typeof result.job_id !== 'string') {
    throw new Error('merge response is invalid');
  }
  return result as unknown as RecordingJobStartResponse;
};

export const finalizeIncompleteRecording = async (payload: {
  channel_login: string;
  filename: string;
}): Promise<RecordingJobStartResponse> => {
  const response = await request('/api/recordings/finalize-incomplete', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || typeof result.job_id !== 'string') {
    throw new Error('finalize response is invalid');
  }
  return result as unknown as RecordingJobStartResponse;
};

export const getRecordingJobStatus = async (jobId: string): Promise<RecordingJobStatusResponse> => {
  const response = await request(`/api/recordings/jobs/${encodeURIComponent(jobId)}`);
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || typeof result.job_id !== 'string') {
    throw new Error('recording job status response is invalid');
  }
  return result as unknown as RecordingJobStatusResponse;
};

export const repairRecordingFile = async (payload: {
  channel_login: string;
  filename: string;
}): Promise<RecordingFileEntry> => {
  const response = await request('/api/recordings/repair', {
    body: JSON.stringify(payload),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || !isObject(result.repaired_file)) {
    throw new Error('repair response is invalid');
  }
  return result.repaired_file as unknown as RecordingFileEntry;
};
