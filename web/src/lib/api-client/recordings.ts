import { isObject, safeJson, readError, request } from "./core";
import type {
  RecordingRule,
  RecordingsResponse,
  ActiveRecording,
  RecordingFileEntry,
  RecordingWatchProgress,
  MergeStartResponse,
  MergeStatusResponse,
} from "./types";

export async function getRecordingRules(): Promise<Array<RecordingRule>> {
  const response = await request("/api/recording-rules");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (!isObject(payload) || !Array.isArray(payload.rules)) {
    throw new Error("recording rules payload is invalid");
  }

  return payload.rules as Array<RecordingRule>;
}

export async function upsertRecordingRule(rule: {
  channel_login: string;
  enabled: boolean;
  quality?: string;
  stop_when_offline?: boolean;
  max_duration_minutes?: number | null;
  keep_last_videos?: number | null;
}): Promise<RecordingRule> {
  const response = await request("/api/recording-rules", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(rule),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  return (await safeJson(response)) as RecordingRule;
}

export async function getRecordings(): Promise<RecordingsResponse> {
  const response = await request("/api/recordings");
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }

  const payload = await safeJson(response);
  if (
    !isObject(payload) ||
    !Array.isArray(payload.active) ||
    !Array.isArray(payload.completed) ||
    !Array.isArray(payload.incomplete)
  ) {
    throw new Error("recordings payload is invalid");
  }

  return {
    active: payload.active as Array<ActiveRecording>,
    completed: payload.completed as Array<RecordingFileEntry>,
    incomplete: payload.incomplete as Array<RecordingFileEntry>,
  };
}

export async function startRecording(
  channel_login: string,
  quality?: string,
  stream_title?: string,
): Promise<void> {
  const response = await request("/api/recordings/start", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ channel_login, quality, stream_title }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function stopRecording(channel_login: string): Promise<void> {
  const response = await request("/api/recordings/stop", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ channel_login }),
  });

  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
}

export async function deleteRecordingFile(payload: {
  bucket: "completed" | "incomplete";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/delete", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

export async function pinRecordingFile(payload: {
  bucket: "completed";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/pin", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

export async function unpinRecordingFile(payload: {
  bucket: "completed";
  channel_login: string;
  filename: string;
}): Promise<void> {
  const response = await request("/api/recordings/unpin", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
}

export async function getRecordingWatchProgress(
  channel_login: string,
  filename: string,
): Promise<RecordingWatchProgress> {
  const params = new URLSearchParams({ channel_login, filename });
  const response = await request(`/api/recordings/progress?${params.toString()}`);
  if (!response.ok) {
    const payload = await safeJson(response);
    throw new Error(readError(payload));
  }
  return (await safeJson(response)) as RecordingWatchProgress;
}

export async function saveRecordingWatchProgress(payload: {
  channel_login: string;
  filename: string;
  position_secs: number;
  duration_secs?: number;
  completed?: boolean;
}): Promise<RecordingWatchProgress> {
  const response = await request("/api/recordings/progress", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
  return (await safeJson(response)) as RecordingWatchProgress;
}

export async function mergeRecordingFiles(payload: {
  channel_login: string;
  filenames: string[];
}): Promise<MergeStartResponse> {
  const response = await request("/api/recordings/merge", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || typeof result.job_id !== "string") {
    throw new Error("merge response is invalid");
  }
  return result as unknown as MergeStartResponse;
}

export async function getMergeStatus(jobId: string): Promise<MergeStatusResponse> {
  const response = await request(`/api/recordings/merge/${encodeURIComponent(jobId)}`);
  if (!response.ok) {
    const body = await safeJson(response);
    throw new Error(readError(body));
  }
  const result = await safeJson(response);
  if (!isObject(result) || typeof result.job_id !== "string") {
    throw new Error("merge status response is invalid");
  }
  return result as unknown as MergeStatusResponse;
}
