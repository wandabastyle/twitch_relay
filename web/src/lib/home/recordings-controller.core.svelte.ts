import { pinRecordingFile, repairRecordingFile, unpinRecordingFile } from '$lib/api-client';
import { getCachedRecordings } from '$lib/api-client/recordings';
import type { ActiveRecording, RecordingFileEntry, RecordingRule } from '$lib/api-client/types';
import { readJsError } from '$lib/home/errors';
import {
  buildActiveRecordings,
  clearPendingJobState,
  executeDeleteRecording,
  handleIncompleteJobOutcome,
  loadRules,
  loadState,
  pollIncompleteJob,
  repairRecordingHelper,
  startIncompleteJob,
  toggleAutoRecord,
  toggleManualRecording,
  toggleRecordingPinHelper,
  type PendingDelete,
  type PendingMerge,
  type PendingRecordingJobState,
  type RecordingPinContext,
  type RepairContext,
} from './recordings-controller.state';

type RecordingsMap = Record<string, ActiveRecording | undefined>;
type RulesMap = Record<string, RecordingRule | undefined>;
type ReadonlyFile = Readonly<RecordingFileEntry>;

const MINIMUM_FILE_COUNT = 1;
const INDEX_ZERO = 0;

export interface RecordingsControllerDeps {
  setError: (message: string | null) => void;
}

export interface RecordingsController {
  activeRecordings: RecordingsMap;
  cancelDeleteRecordingFile: () => void;
  cancelProcessIncompleteFiles: () => void;
  clearMergeSelection: () => void;
  completedRecordings: readonly RecordingFileEntry[];
  confirmDeleteRecordingFile: () => Promise<void>;
  confirmProcessIncompleteFiles: () => Promise<void>;
  deletingRecordingKey: string | undefined;
  incompleteRecordings: readonly RecordingFileEntry[];
  loadRecordingRules: () => Promise<void>;
  loadRecordingState: () => Promise<void>;
  mergingRecordingKey: string | undefined;
  pendingDelete: PendingDelete | undefined;
  pendingJob: PendingRecordingJobState | undefined;
  pendingMerge: PendingMerge | undefined;
  pinningRecordingKey: string | undefined;
  recordingRules: RulesMap;
  repairRecording: (file: ReadonlyFile) => Promise<void>;
  repairingRecordingKey: string | undefined;
  requestDeleteRecordingFile: (bucket: 'completed' | 'incomplete', file: ReadonlyFile) => void;
  requestProcessIncompleteFiles: (channelLogin: string) => void;
  selectedIncompleteFilenames: Set<string>;
  selectedQuality: (channelLogin: string) => string;
  toggleAutoRecord: (channelLogin: string) => Promise<void>;
  toggleIncompleteMergeSelection: (filename: string) => void;
  toggleManualRecording: (channelLogin: string, quality: string, title?: string) => Promise<void>;
  toggleRecordingPin: (file: ReadonlyFile) => Promise<void>;
}

interface ProcessCtx {
  loadStateFn: () => Promise<void>;
  setError: (msg: string | null) => void;
  setMergingKey: (channelKey: string | undefined) => void;
  setPendingJob: (value: PendingRecordingJobState | undefined) => void;
  setPendingMerge: (value: PendingMerge | undefined) => void;
}

const processIncompleteFiles = async (
  merge: PendingMerge | undefined,
  ctx: ProcessCtx,
): Promise<void> => {
  if (!merge) {
    return;
  }
  ctx.setMergingKey(merge.channelLogin);
  ctx.setError(null);
  try {
    const startResponse = await startIncompleteJob(merge);
    const status = await pollIncompleteJob(startResponse, ctx.setPendingJob);
    clearPendingJobState(ctx.setPendingJob, ctx.setPendingMerge);
    await handleIncompleteJobOutcome(status, ctx.setError, ctx.loadStateFn);
  } catch (error) {
    clearPendingJobState(ctx.setPendingJob, ctx.setPendingMerge);
    ctx.setError(readJsError(error, 'failed to process recordings'));
  } finally {
    ctx.setMergingKey(undefined);
  }
};

export const createRecordingsController = (
  deps: Readonly<RecordingsControllerDeps>,
): RecordingsController => {
  const cachedRecordings = getCachedRecordings();
  const initialActiveRecordings = cachedRecordings?.active ?? [];
  const initialCompletedRecordings = cachedRecordings?.completed ?? [];
  const initialIncompleteRecordings = cachedRecordings?.incomplete ?? [];

  let recordingRules = $state<RulesMap>({});
  let activeRecordings = $state<RecordingsMap>(
    buildActiveRecordings(initialActiveRecordings)
  );
  let completedRecordings = $state<readonly RecordingFileEntry[]>(
    initialCompletedRecordings
  );
  let incompleteRecordings = $state<readonly RecordingFileEntry[]>(
    initialIncompleteRecordings
  );
  let deletingRecordingKey = $state<string | undefined>();
  let pinningRecordingKey = $state<string | undefined>();
  let mergingRecordingKey = $state<string | undefined>();
  let selectedIncompleteFilenames = $state<Set<string>>(new Set());
  let pendingJob = $state<PendingRecordingJobState | undefined>();
  let repairingRecordingKey = $state<string | undefined>();
  let pendingDelete = $state<PendingDelete | undefined>();
  let pendingMerge = $state<PendingMerge | undefined>();
  const { setError } = deps;

  const selectedQuality = (channelLogin: string): string =>
    recordingRules[channelLogin]?.quality ?? 'best';

  const loadRecordingRules = async (): Promise<void> => {
    await loadRules((rules) => {
      recordingRules = rules;
    });
  };

  const loadRecordingState = async (): Promise<void> => {
    await loadState({
      setActive: (recordings) => {
        activeRecordings = recordings;
      },
      setCompleted: (recordings) => {
        completedRecordings = recordings;
      },
      setIncomplete: (recordings) => {
        incompleteRecordings = recordings;
      },
      setSelected: (selection) => {
        selectedIncompleteFilenames = selection;
      },
    });
  };

  const requestDeleteRecordingFile = (
    bucket: 'completed' | 'incomplete',
    file: ReadonlyFile,
  ): void => {
    pendingDelete = { bucket, file };
  };

  const confirmDeleteRecordingFile = async (): Promise<void> => {
    if (!pendingDelete) {
      return;
    }
    await executeDeleteRecording({
      clearPending: () => {
        pendingDelete = undefined;
      },
      loadRecordingState,
      pendingDelete,
      setDeletingKey: (channelKey: string | undefined) => {
        deletingRecordingKey = channelKey;
      },
      setError,
    });
  };

  const cancelDeleteRecordingFile = (): void => {
    pendingDelete = undefined;
  };

  const pinCtx = (): RecordingPinContext => ({
    loadRecordingState,
    setError,
    setPinningKey: (channelKey: string | undefined): void => {
      pinningRecordingKey = channelKey;
    },
  });

  const toggleRecordingPin = async (file: ReadonlyFile): Promise<void> => {
    await toggleRecordingPinHelper({
      ctx: pinCtx(),
      file,
      pinFn: pinRecordingFile,
      unpinFn: unpinRecordingFile,
    });
  };

  const repairCtx = (): RepairContext => ({
    loadRecordingState,
    setError,
    setRepairingKey: (channelKey: string | undefined): void => {
      repairingRecordingKey = channelKey;
    },
  });

  const repairRecording = async (file: ReadonlyFile): Promise<void> => {
    await repairRecordingHelper({
      ctx: repairCtx(),
      file,
      repairFn: async (params: { channel_login: string; filename: string }): Promise<void> => {
        await repairRecordingFile(params);
      },
    });
  };

  const toggleIncompleteMergeSelection = (filename: string): void => {
    const next = new Set(selectedIncompleteFilenames);
    if (next.has(filename)) {
      next.delete(filename);
    } else {
      next.add(filename);
    }
    selectedIncompleteFilenames = next;
  };

  const clearMergeSelection = (): void => {
    selectedIncompleteFilenames = new Set();
  };

  const requestProcessIncompleteFiles = (channelLogin: string): void => {
    const selected = [...selectedIncompleteFilenames];
    if (selected.length === INDEX_ZERO) {
      setError('Please select at least 1 file to process');
      return;
    }
    pendingMerge = {
      action: selected.length === MINIMUM_FILE_COUNT ? 'finalize' : 'merge',
      channelLogin,
      filenames: selected,
    };
  };

  const confirmProcessIncompleteFiles = async (): Promise<void> => {
    await processIncompleteFiles(pendingMerge, {
      loadStateFn: loadRecordingState,
      setError,
      setMergingKey: (channelKey: string | undefined) => {
        mergingRecordingKey = channelKey;
      },
      setPendingJob: (value: PendingRecordingJobState | undefined) => {
        pendingJob = value;
      },
      setPendingMerge: (value: PendingMerge | undefined) => {
        pendingMerge = value;
      },
    });
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
    get completedRecordings() {
      return completedRecordings;
    },
    confirmDeleteRecordingFile,
    confirmProcessIncompleteFiles,
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
    toggleAutoRecord: async (login): Promise<void> => {
      await toggleAutoRecord({
        channelLogin: login,
        loadRulesFn: loadRecordingRules,
        qualityFn: selectedQuality,
        rules: recordingRules,
        setError,
      });
    },
    toggleIncompleteMergeSelection,
    toggleManualRecording: async (login, quality, title): Promise<void> => {
      await toggleManualRecording({
        active: activeRecordings,
        channelLogin: login,
        loadStateFn: loadRecordingState,
        quality,
        setError,
        title,
      });
    },
    toggleRecordingPin,
  };
};
