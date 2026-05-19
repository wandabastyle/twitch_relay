import {
  deleteRecordingFile,
  finalizeIncompleteRecording,
  getRecordingJobStatus,
  getRecordingRules,
  getRecordings,
  mergeRecordingFiles,
  pinRecordingFile,
  repairRecordingFile,
  startRecording,
  stopRecording,
  unpinRecordingFile,
  upsertRecordingRule,
} from '$lib/api-client';
import { readJsError } from '$lib/home/errors';
import type {
  ActiveRecording,
  RecordingFileEntry,
  RecordingJobKind,
  RecordingJobStartResponse,
  RecordingJobStatusResponse,
  RecordingRule,
} from '$lib/api-client/types';

const MINIMUM_FILE_COUNT = 1;
const POLLING_INTERVAL_MS = 1_500;
const POLLING_TIMEOUT_MS = 10 * 60 * 1_000;
const INDEX_ZERO = 0;

interface PendingRecordingJobState {
  channelLogin: string;
  expectedFilename: string;
  jobId: string;
  kind: RecordingJobKind;
  sourceCount: number;
  status: RecordingJobStatusResponse['status'];
}

type PendingDelete = {
  bucket: 'completed' | 'incomplete';
  file: RecordingFileEntry;
};

type PendingMerge = {
  action: 'finalize' | 'merge';
  channelLogin: string;
  filenames: string[];
};

export interface RecordingsControllerDeps {
  setError: (message: string | undefined) => void;
}

export interface RecordingsController {
  activeRecordings: Record<string, ActiveRecording>;
  cancelDeleteRecordingFile: () => void;
  cancelProcessIncompleteFiles: () => void;
  clearMergeSelection: () => void;
  completedRecordings: RecordingFileEntry[];
  confirmDeleteRecordingFile: () => Promise<void>;
  confirmProcessIncompleteFiles: () => Promise<void>;
  deletingRecordingKey: string | undefined;
  incompleteRecordings: RecordingFileEntry[];
  loadRecordingRules: () => Promise<void>;
  loadRecordingState: () => Promise<void>;
  mergingRecordingKey: string | undefined;
  pendingDelete: PendingDelete | undefined;
  pendingJob: PendingRecordingJobState | undefined;
  pendingMerge: PendingMerge | undefined;
  pinningRecordingKey: string | undefined;
  recordingRules: Record<string, RecordingRule>;
  repairRecording: (file: RecordingFileEntry) => Promise<void>;
  repairingRecordingKey: string | undefined;
  requestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
  requestProcessIncompleteFiles: (channelLogin: string) => void;
  selectedIncompleteFilenames: Set<string>;
  selectedQuality: (channelLogin: string) => string;
  toggleAutoRecord: (channelLogin: string) => Promise<void>;
  toggleIncompleteMergeSelection: (filename: string) => void;
  toggleManualRecording: (channelLogin: string, quality: string, title?: string) => Promise<void>;
  toggleRecordingPin: (file: RecordingFileEntry) => Promise<void>;
}

export const createRecordingsController = (
  deps: RecordingsControllerDeps,
): RecordingsController => {
  let recordingRules = $state<Record<string, RecordingRule>>({});
  let activeRecordings = $state<Record<string, ActiveRecording>>({});
  let completedRecordings = $state<RecordingFileEntry[]>([]);
  let incompleteRecordings = $state<RecordingFileEntry[]>([]);
  let deletingRecordingKey = $state<string | undefined>(undefined);
  let pinningRecordingKey = $state<string | undefined>(undefined);
  let mergingRecordingKey = $state<string | undefined>(undefined);
  let selectedIncompleteFilenames = $state<Set<string>>(new Set());
  let pendingJob = $state<PendingRecordingJobState | undefined>(undefined);
  let repairingRecordingKey = $state<string | undefined>(undefined);
  let pendingDelete = $state<PendingDelete | undefined>(undefined);
  let pendingMerge = $state<PendingMerge | undefined>(undefined);

  const { setError } = deps;

  const loadRecordingRules = async (): Promise<void> => {
    try {
      const rules = await getRecordingRules();
      const next: Record<string, RecordingRule> = {};
      for (const rule of rules) {
        next[rule.channel_login] = rule;
      }
      recordingRules = next;
    } catch {
      // ignore transient rule loading failures
    }
  };

  const loadRecordingState = async (): Promise<void> => {
    try {
      const recordings = await getRecordings();
      const next: Record<string, ActiveRecording> = {};
      for (const recording of recordings.active) {
        next[recording.channel_login] = recording;
      }
      activeRecordings = next;
      completedRecordings = recordings.completed;
      incompleteRecordings = recordings.incomplete;
      selectedIncompleteFilenames = new Set(); // Clear selection on reload
    } catch {
      // ignore transient recording state failures
    }
  };

  const selectedQuality = (channelLogin: string): string => {
    const rule = recordingRules[channelLogin];
    return rule?.quality ? rule.quality : 'best';
  };

  const toggleAutoRecord = async (channelLogin: string): Promise<void> => {
    const current = recordingRules[channelLogin];
    const enabled = !current?.enabled;
    try {
      await upsertRecordingRule({
        channel_login: channelLogin,
        enabled,
        keep_last_videos: current?.keep_last_videos ?? null,
        max_duration_minutes: current?.max_duration_minutes ?? null,
        quality: selectedQuality(channelLogin),
        stop_when_offline: current?.stop_when_offline ?? true,
      });
      await loadRecordingRules();
    } catch (error) {
      setError(readJsError(error, 'failed to toggle auto-record'));
    }
  };

  const toggleManualRecording = async (
    channelLogin: string,
    quality: string,
    title?: string,
  ): Promise<void> => {
    const active = activeRecordings[channelLogin];
    try {
      const recordingPromise = active
        ? stopRecording(channelLogin)
        : startRecording(channelLogin, quality, title);
      await recordingPromise;
      await loadRecordingState();
    } catch (error) {
      setError(readJsError(error, 'failed to toggle recording'));
    }
  };

  const requestDeleteRecordingFile = (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ): void => {
    pendingDelete = { bucket, file };
  };

  const confirmDeleteRecordingFile = async (): Promise<void> => {
    if (!pendingDelete) {
      return;
    }

    const { bucket, file } = pendingDelete;
    const key = `${bucket}:${file.channel_login}:${file.filename}`;
    deletingRecordingKey = key;
    setError(undefined);

    try {
      await deleteRecordingFile({
        bucket,
        channel_login: file.channel_login,
        filename: file.filename,
      });
      await loadRecordingState();
    } catch (error) {
      setError(readJsError(error, 'failed to delete recording'));
    } finally {
      deletingRecordingKey = undefined;
      pendingDelete = undefined;
    }
  };

  const cancelDeleteRecordingFile = (): void => {
    pendingDelete = undefined;
  };

  const toggleRecordingPin = async (file: RecordingFileEntry): Promise<void> => {
    const key = `completed:${file.channel_login}:${file.filename}`;
    pinningRecordingKey = key;
    setError(undefined);

    try {
      const pinPromise = file.pinned
        ? unpinRecordingFile({
            bucket: 'completed',
            channel_login: file.channel_login,
            filename: file.filename,
          })
        : pinRecordingFile({
            bucket: 'completed',
            channel_login: file.channel_login,
            filename: file.filename,
          });
      await pinPromise;
      await loadRecordingState();
    } catch (error) {
      const errorMessage = file.pinned ? 'failed to unpin recording' : 'failed to pin recording';
      setError(readJsError(error, errorMessage));
    } finally {
      pinningRecordingKey = undefined;
    }
  };

  const repairRecording = async (file: RecordingFileEntry): Promise<void> => {
    const key = `completed:${file.channel_login}:${file.filename}`;
    repairingRecordingKey = key;
    setError(undefined);
    try {
      await repairRecordingFile({
        channel_login: file.channel_login,
        filename: file.filename,
      });
      await loadRecordingState();
    } catch (error) {
      setError(readJsError(error, 'failed to repair recording'));
    } finally {
      repairingRecordingKey = undefined;
    }
  };

  const toggleIncompleteMergeSelection = (filename: string): void => {
    const newSelection = new Set(selectedIncompleteFilenames);
    if (newSelection.has(filename)) {
      newSelection.delete(filename);
    } else {
      newSelection.add(filename);
    }
    selectedIncompleteFilenames = newSelection;
  };

  const clearMergeSelection = (): void => {
    selectedIncompleteFilenames = new Set();
  };

  const requestProcessIncompleteFiles = (channelLogin: string): void => {
    const selectedFiles = Array.from(selectedIncompleteFilenames);
    if (selectedFiles.length === INDEX_ZERO) {
      setError('Please select at least 1 file to process');
      return;
    }

    const action: PendingMerge['action'] =
      selectedFiles.length === MINIMUM_FILE_COUNT ? 'finalize' : 'merge';
    pendingMerge = { action, channelLogin, filenames: selectedFiles };
  };

  const confirmProcessIncompleteFiles = async (): Promise<void> => {
    if (!pendingMerge) {
      return;
    }

    const { channelLogin, action, filenames } = pendingMerge;
    mergingRecordingKey = channelLogin;
    setError(undefined);

    try {
      const startResponse: RecordingJobStartResponse =
        action === 'finalize'
          ? await finalizeIncompleteRecording({
              channel_login: channelLogin,
              filename: filenames[0] ?? '',
            })
          : await mergeRecordingFiles({
              channel_login: channelLogin,
              filenames,
            });

      pendingJob = {
        channelLogin: startResponse.channel_login,
        expectedFilename: startResponse.expected_filename,
        jobId: startResponse.job_id,
        kind: startResponse.kind,
        sourceCount: startResponse.source_count,
        status: 'queued',
      };

      const startedAt = Date.now();
      while (Date.now() - startedAt < POLLING_TIMEOUT_MS) {
        await new Promise((resolve) => setTimeout(resolve, POLLING_INTERVAL_MS));
        const status = await getRecordingJobStatus(startResponse.job_id);
        pendingJob = {
          channelLogin: status.channel_login,
          expectedFilename: status.expected_filename,
          jobId: status.job_id,
          kind: status.kind,
          sourceCount: startResponse.source_count,
          status: status.status,
        };

        if (status.status === 'completed') {
          pendingJob = undefined;
          pendingMerge = undefined;
          await loadRecordingState();
          return;
        }
        if (status.status === 'failed') {
          pendingJob = undefined;
          pendingMerge = undefined;
          const errorMessage = status.error ?? 'recording job failed';
          setError(errorMessage);
          return;
        }
      }

      pendingJob = undefined;
      pendingMerge = undefined;
      setError('recording job polling timed out');
    } catch (error) {
      pendingJob = undefined;
      pendingMerge = undefined;
      setError(readJsError(error, 'failed to process recordings'));
    } finally {
      mergingRecordingKey = undefined;
    }
  };

  const cancelProcessIncompleteFiles = (): void => {
    pendingMerge = undefined;
  };

  return {
    get activeRecordings() {
      return activeRecordings;
    },
    cancelDeleteRecordingFile,
    cancelProcessIncompleteFiles,
    clearMergeSelection,
    confirmProcessIncompleteFiles,
    get completedRecordings() {
      return completedRecordings;
    },
    confirmDeleteRecordingFile,
    get deletingRecordingKey() {
      return deletingRecordingKey;
    },
    get incompleteRecordings() {
      return incompleteRecordings;
    },
    loadRecordingRules,
    loadRecordingState,
    get mergingRecordingKey() {
      return mergingRecordingKey;
    },
    get pendingDelete() {
      return pendingDelete;
    },
    get pendingJob() {
      return pendingJob;
    },
    get pendingMerge() {
      return pendingMerge;
    },
    get pinningRecordingKey() {
      return pinningRecordingKey;
    },
    get recordingRules() {
      return recordingRules;
    },
    repairRecording,
    get repairingRecordingKey() {
      return repairingRecordingKey;
    },
    requestDeleteRecordingFile,
    requestProcessIncompleteFiles,
    get selectedIncompleteFilenames() {
      return selectedIncompleteFilenames;
    },
    selectedQuality,
    toggleAutoRecord,
    toggleIncompleteMergeSelection,
    toggleManualRecording,
    toggleRecordingPin,
  };
};
