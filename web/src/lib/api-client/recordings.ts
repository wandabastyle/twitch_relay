import { isObject, readApiError, request, safeJson } from './core.js';
import type {
  ActiveRecording,
  RecordingFileEntry,
  RecordingJobStartResponse,
  RecordingJobStatusResponse,
  RecordingRule,
  RecordingWatchProgress,
  RecordingsResponse,
} from './types.js';

const isRecordingRule = (value: unknown): value is RecordingRule =>
  isObject(value) &&
  typeof value.channel_login === 'string' &&
  typeof value.enabled === 'boolean' &&
  typeof value.quality === 'string' &&
  typeof value.stop_when_offline === 'boolean' &&
  (value.max_duration_minutes === null || typeof value.max_duration_minutes === 'number') &&
  (value.keep_last_videos === null || typeof value.keep_last_videos === 'number');

const isActiveRecording = (value: unknown): value is ActiveRecording =>
  isObject(value) &&
  typeof value.channel_login === 'string' &&
  typeof value.quality === 'string' &&
  typeof value.started_at_unix === 'number' &&
  typeof value.output_path === 'string' &&
  (value.mode === 'manual' || value.mode === 'auto');

const isRecordingFileEntry = (value: unknown): value is RecordingFileEntry =>
  isObject(value) &&
  typeof value.channel_login === 'string' &&
  typeof value.filename === 'string' &&
  typeof value.path_display === 'string' &&
  typeof value.status === 'string' &&
  typeof value.pinned === 'boolean' &&
  typeof value.has_hls === 'boolean' &&
  (value.processing_state === 'processing' || value.processing_state === 'ready');

const isRecordingWatchProgress = (value: unknown): value is RecordingWatchProgress =>
  isObject(value) &&
  typeof value.channel_login === 'string' &&
  typeof value.filename === 'string' &&
  (value.position_secs === null || typeof value.position_secs === 'number') &&
  (value.duration_secs === null || typeof value.duration_secs === 'number') &&
  (value.updated_at_unix === null || typeof value.updated_at_unix === 'number') &&
  typeof value.completed === 'boolean';

const isRecordingJobStartResponse = (value: unknown): value is RecordingJobStartResponse =>
  isObject(value) &&
  typeof value.job_id === 'string' &&
  (value.kind === 'merge' || value.kind === 'finalize') &&
  typeof value.channel_login === 'string' &&
  typeof value.expected_filename === 'string' &&
  typeof value.source_count === 'number';

const isRecordingJobStatusResponse = (value: unknown): value is RecordingJobStatusResponse =>
  isObject(value) &&
  typeof value.job_id === 'string' &&
  (value.kind === 'merge' || value.kind === 'finalize') &&
  typeof value.status === 'string' &&
  typeof value.channel_login === 'string' &&
  typeof value.expected_filename === 'string' &&
  (value.final_filename === null || typeof value.final_filename === 'string') &&
  (value.error === null || typeof value.error === 'string');

export const getRecordingRules = async (): Promise<readonly RecordingRule[]> => {
  const response = await request('/api/recording-rules');
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readApiError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !Array.isArray(payload.rules) ||
    !payload.rules.every(isRecordingRule)
  ) {
    throw new Error('recording rules payload is invalid');
  }

  return payload.rules;
};

export const upsertRecordingRule = async (rule: {
  readonly channel_login: string;
  readonly enabled: boolean;
  readonly keep_last_videos?: number | null;
  readonly max_duration_minutes?: number | null;
  readonly quality?: string;
  readonly stop_when_offline?: boolean;
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

  const result = await safeJson(response);
  if (!isRecordingRule(result)) {
    throw new Error('upsert recording rule response is invalid');
  }
  return result;
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

  if (
    !Array.isArray(payload.active) ||
    !payload.active.every(isActiveRecording) ||
    !Array.isArray(payload.completed) ||
    !payload.completed.every(isRecordingFileEntry) ||
    !Array.isArray(payload.incomplete) ||
    !payload.incomplete.every(isRecordingFileEntry)
  ) {
    throw new Error('recordings payload contains invalid data');
  }

  return {
    active: payload.active,
    completed: payload.completed,
    incomplete: payload.incomplete,
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
  readonly bucket: 'completed' | 'incomplete';
  readonly channel_login: string;
  readonly filename: string;
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
  readonly bucket: 'completed';
  readonly channel_login: string;
  readonly filename: string;
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
  readonly bucket: 'completed';
  readonly channel_login: string;
  readonly filename: string;
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
  const result = await safeJson(response);
  if (!isRecordingWatchProgress(result)) {
    throw new Error('recording watch progress response is invalid');
  }
  return result;
};

export const saveRecordingWatchProgress = async (payload: {
  readonly channel_login: string;
  readonly completed?: boolean;
  readonly duration_secs?: number;
  readonly filename: string;
  readonly position_secs: number;
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
  const result = await safeJson(response);
  if (!isRecordingWatchProgress(result)) {
    throw new Error('save recording watch progress response is invalid');
  }
  return result;
};

export const mergeRecordingFiles = async (payload: {
  readonly channel_login: string;
  readonly filenames: readonly string[];
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
  if (!isRecordingJobStartResponse(result)) {
    throw new Error('merge response is invalid');
  }
  return result;
};

export const finalizeIncompleteRecording = async (payload: {
  readonly channel_login: string;
  readonly filename: string;
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
  if (!isRecordingJobStartResponse(result)) {
    throw new Error('finalize response is invalid');
  }
  return result;
};

export const getRecordingJobStatus = async (jobId: string): Promise<RecordingJobStatusResponse> => {
  const response = await request(`/api/recordings/jobs/${encodeURIComponent(jobId)}`);
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readApiError(body));
  }
  const result = await safeJson(response);
  if (!isRecordingJobStatusResponse(result)) {
    throw new Error('recording job status response is invalid');
  }
  return result;
};

export const repairRecordingFile = async (payload: {
  readonly channel_login: string;
  readonly filename: string;
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
  if (!isObject(result) || !isRecordingFileEntry(result.repaired_file)) {
    throw new Error('repair response is invalid');
  }
  return result.repaired_file;
};
