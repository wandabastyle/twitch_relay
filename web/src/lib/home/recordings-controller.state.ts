import {
  deleteRecordingFile,
  finalizeIncompleteRecording,
  getRecordingJobStatus,
  getRecordingRules,
  getRecordings,
  mergeRecordingFiles,
  startRecording,
  stopRecording,
  upsertRecordingRule,
} from '$lib/api-client';
import type {
  ActiveRecording,
  RecordingFileEntry,
  RecordingJobStartResponse,
  RecordingJobStatusResponse,
  RecordingRule,
} from '$lib/api-client/types';
import { readJsError } from '$lib/home/errors';

const INDEX_ZERO = 0;
const MILLISECONDS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const POLLING_INTERVAL_MS = 1500;
const POLLING_TIMEOUT_MINUTES = 10;
const POLLING_TIMEOUT_MS = POLLING_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MILLISECONDS_PER_SECOND;

type ReadonlyFile = Readonly<RecordingFileEntry>;
type RecordingsMap = Record<string, ActiveRecording | undefined>;
type RulesMap = Record<string, RecordingRule | undefined>;

export interface PendingRecordingJobState {
  channelLogin: string;
  expectedFilename: string;
  jobId: string;
  kind: 'finalize' | 'merge';
  sourceCount: number;
  status: RecordingJobStatusResponse['status'];
}

export interface PendingDelete {
  bucket: 'completed' | 'incomplete';
  file: ReadonlyFile;
}

export interface PendingMerge {
  action: 'finalize' | 'merge';
  channelLogin: string;
  filenames: readonly string[];
}

export const buildActiveRecordings = (
  active: readonly Readonly<ActiveRecording>[],
): RecordingsMap => {
  const next: RecordingsMap = {};
  for (const recording of active) {
    next[recording.channel_login] = recording;
  }
  return next;
};

const buildRecordingRules = (rules: readonly Readonly<RecordingRule>[]): RulesMap => {
  const next: RulesMap = {};
  for (const rule of rules) {
    next[rule.channel_login] = rule;
  }
  return next;
};

interface StateSetters {
  setActive: (value: RecordingsMap) => void;
  setCompleted: (value: readonly RecordingFileEntry[]) => void;
  setIncomplete: (value: readonly RecordingFileEntry[]) => void;
  setSelected: (value: Set<string>) => void;
}

export const loadState = async (setters: StateSetters): Promise<void> => {
  try {
    const recordings = await getRecordings();
    setters.setActive(buildActiveRecordings(recordings.active));
    setters.setCompleted(recordings.completed);
    setters.setIncomplete(recordings.incomplete);
    setters.setSelected(new Set());
  } catch {
    // Intentionally ignored to preserve existing silent failure behavior.
  }
};

export const loadRules = async (setRules: (rules: RulesMap) => void): Promise<void> => {
  try {
    setRules(buildRecordingRules(await getRecordingRules()));
  } catch {
    // Intentionally ignored to preserve existing silent failure behavior.
  }
};

interface AutoRecordParams {
  channelLogin: string;
  rules: RulesMap;
  qualityFn: (login: string) => string;
  loadRulesFn: () => Promise<void>;
  setError: (msg: string | null) => void;
}

export const toggleAutoRecord = async (params: AutoRecordParams): Promise<void> => {
  const { channelLogin, rules, qualityFn, loadRulesFn, setError } = params;
  const current = rules[channelLogin];
  try {
    await upsertRecordingRule({
      channel_login: channelLogin,
      enabled: !(current?.enabled ?? false),
      keep_last_videos: current?.keep_last_videos ?? null,
      max_duration_minutes: current?.max_duration_minutes ?? null,
      quality: qualityFn(channelLogin),
      stop_when_offline: current?.stop_when_offline ?? true,
    });
    await loadRulesFn();
  } catch (error) {
    setError(readJsError(error, 'failed to toggle auto-record'));
  }
};

interface ManualRecordingParams {
  channelLogin: string;
  quality: string;
  title: string | undefined;
  active: RecordingsMap;
  loadStateFn: () => Promise<void>;
  setError: (msg: string | null) => void;
}

export const toggleManualRecording = async (params: ManualRecordingParams): Promise<void> => {
  const { channelLogin, quality, title, active, loadStateFn, setError } = params;
  try {
    await (active[channelLogin] === undefined
      ? startRecording(channelLogin, quality, title)
      : stopRecording(channelLogin));
    await loadStateFn();
  } catch (error) {
    setError(readJsError(error, 'failed to toggle recording'));
  }
};

const waitPollingInterval = async (): Promise<void> => {
  await new Promise<void>((resolve) => {
    setTimeout(resolve, POLLING_INTERVAL_MS);
  });
};

interface PollStatusUpdate {
  channelLogin: string;
  expectedFilename: string;
  jobId: string;
  kind: 'finalize' | 'merge';
  sourceCount: number;
  status: RecordingJobStatusResponse['status'];
}

const createStatusUpdate = (
  status: RecordingJobStatusResponse,
  sourceCount: number,
): PollStatusUpdate => ({
  channelLogin: status.channel_login,
  expectedFilename: status.expected_filename,
  jobId: status.job_id,
  kind: status.kind,
  sourceCount,
  status: status.status,
});

const pollUntilDone = async (
  jobId: string,
  sourceCount: number,
  startedAt: number,
  setPendingJob: (value: Readonly<PendingRecordingJobState> | undefined) => void,
): Promise<RecordingJobStatusResponse | undefined> => {
  if (Date.now() - startedAt >= POLLING_TIMEOUT_MS) {
    return undefined;
  }
  await waitPollingInterval();
  const status = await getRecordingJobStatus(jobId);
  setPendingJob(createStatusUpdate(status, sourceCount));
  return status.status === 'completed' || status.status === 'failed'
    ? status
    : pollUntilDone(jobId, sourceCount, startedAt, setPendingJob);
};

export const startIncompleteJob = async (
  merge: Readonly<PendingMerge>,
): Promise<RecordingJobStartResponse> => {
  if (merge.action === 'merge') {
    const response = await mergeRecordingFiles({
      channel_login: merge.channelLogin,
      filenames: [...merge.filenames],
    });
    return response;
  }
  const response = await finalizeIncompleteRecording({
    channel_login: merge.channelLogin,
    filename: merge.filenames[INDEX_ZERO] ?? '',
  });
  return response;
};

export const pollIncompleteJob = async (
  startResponse: Readonly<RecordingJobStartResponse>,
  setPendingJob: (value: Readonly<PendingRecordingJobState> | undefined) => void,
): Promise<RecordingJobStatusResponse | undefined> => {
  setPendingJob({
    channelLogin: startResponse.channel_login,
    expectedFilename: startResponse.expected_filename,
    jobId: startResponse.job_id,
    kind: startResponse.kind,
    sourceCount: startResponse.source_count,
    status: 'queued',
  });

  const status = await pollUntilDone(
    startResponse.job_id,
    startResponse.source_count,
    Date.now(),
    setPendingJob,
  );
  return status;
};

export const handleIncompleteJobOutcome = async (
  status: Readonly<RecordingJobStatusResponse> | undefined,
  setError: (message: string | null) => void,
  loadRecordingState: () => Promise<void>,
): Promise<void> => {
  if (!status) {
    setError('recording job polling timed out');
    return;
  }
  if (status.status === 'failed') {
    setError(status.error ?? 'recording job failed');
    return;
  }
  await loadRecordingState();
};

const buildDeleteKey = (bucket: string, channelLogin: string, filename: string): string =>
  `${bucket}:${channelLogin}:${filename}`;

interface DeleteRecordingParams {
  pendingDelete: Readonly<PendingDelete>;
  loadRecordingState: () => Promise<void>;
  setError: (msg: string | null) => void;
  setDeletingKey: (key: string | undefined) => void;
  clearPending: () => void;
}

export const executeDeleteRecording = async (params: DeleteRecordingParams): Promise<void> => {
  const { pendingDelete, loadRecordingState, setError, setDeletingKey, clearPending } = params;
  setDeletingKey(
    buildDeleteKey(
      pendingDelete.bucket,
      pendingDelete.file.channel_login,
      pendingDelete.file.filename,
    ),
  );
  setError(null);
  try {
    await deleteRecordingFile({
      bucket: pendingDelete.bucket,
      channel_login: pendingDelete.file.channel_login,
      filename: pendingDelete.file.filename,
    });
    await loadRecordingState();
  } catch (error) {
    setError(readJsError(error, 'failed to delete recording'));
  } finally {
    setDeletingKey(undefined);
    clearPending();
  }
};

export const clearPendingJobState = (
  setPendingJob: (value: Readonly<PendingRecordingJobState> | undefined) => void,
  setPendingMerge: (value: Readonly<PendingMerge> | undefined) => void,
): void => {
  setPendingJob(undefined);
  setPendingMerge(undefined);
};

export interface RecordingPinContext {
  loadRecordingState: () => Promise<void>;
  setError: (message: string | null) => void;
  setPinningKey: (key: string | undefined) => void;
}

const buildPinKey = (file: ReadonlyFile): string =>
  `completed:${file.channel_login}:${file.filename}`;

interface TogglePinParams {
  file: ReadonlyFile;
  ctx: RecordingPinContext;
  pinFn: (params: {
    bucket: 'completed';
    channel_login: string;
    filename: string;
  }) => Promise<void>;
  unpinFn: (params: {
    bucket: 'completed';
    channel_login: string;
    filename: string;
  }) => Promise<void>;
}

export const toggleRecordingPinHelper = async (params: TogglePinParams): Promise<void> => {
  const { file, ctx, pinFn, unpinFn } = params;
  ctx.setPinningKey(buildPinKey(file));
  ctx.setError(null);
  try {
    const request = file.pinned ? unpinFn : pinFn;
    await request({
      bucket: 'completed',
      channel_login: file.channel_login,
      filename: file.filename,
    });
    await ctx.loadRecordingState();
  } catch (error) {
    ctx.setError(
      readJsError(error, file.pinned ? 'failed to unpin recording' : 'failed to pin recording'),
    );
  } finally {
    ctx.setPinningKey(undefined);
  }
};

export interface RepairContext {
  loadRecordingState: () => Promise<void>;
  setError: (message: string | null) => void;
  setRepairingKey: (key: string | undefined) => void;
}

const buildRepairKey = (file: ReadonlyFile): string =>
  `completed:${file.channel_login}:${file.filename}`;

interface RepairRecordingParams {
  file: ReadonlyFile;
  ctx: RepairContext;
  repairFn: (params: { channel_login: string; filename: string }) => Promise<void>;
}

export const repairRecordingHelper = async (params: RepairRecordingParams): Promise<void> => {
  const { file, ctx, repairFn } = params;
  ctx.setRepairingKey(buildRepairKey(file));
  ctx.setError(null);
  try {
    await repairFn({ channel_login: file.channel_login, filename: file.filename });
    await ctx.loadRecordingState();
  } catch (error) {
    ctx.setError(readJsError(error, 'failed to repair recording'));
  } finally {
    ctx.setRepairingKey(undefined);
  }
};
