import { isObject, readApiError, request, safeJson } from './core.js';
import type {
  RecordingFileEntry,
  RecordingJobStartResponse,
  RecordingJobStatusResponse,
} from './types.js';

const isRecordingFileEntry = (value: unknown): value is RecordingFileEntry =>
  isObject(value) &&
  typeof value.channel_login === 'string' &&
  typeof value.filename === 'string' &&
  typeof value.path_display === 'string' &&
  typeof value.status === 'string' &&
  typeof value.pinned === 'boolean' &&
  typeof value.has_hls === 'boolean' &&
  (value.processing_state === 'processing' || value.processing_state === 'ready');

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
    throw new Error(readApiError(await safeJson(response)));
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
    throw new Error(readApiError(await safeJson(response)));
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
    throw new Error(readApiError(await safeJson(response)));
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
    throw new Error(readApiError(await safeJson(response)));
  }
  const result = await safeJson(response);
  if (!isObject(result) || !isRecordingFileEntry(result.repaired_file)) {
    throw new Error('repair response is invalid');
  }
  return result.repaired_file;
};
