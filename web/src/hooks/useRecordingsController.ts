import { useCallback, useState } from 'react';
import { pinRecordingFile, repairRecordingFile, unpinRecordingFile } from '../api-client';
import { getCachedRecordings } from '../api-client/recordings';
import type { ActiveRecording, RecordingFileEntry, RecordingRule } from '../api-client/types';
import { readJsError } from './errors';
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
} from './recordings-controller-state';

export interface RecordingsControllerDeps {
  setError: (message: string | null) => void;
}

export interface RecordingsController {
  activeRecordings: Record<string, ActiveRecording | undefined>;
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
  recordingRules: Record<string, RecordingRule | undefined>;
  repairRecording: (file: Readonly<RecordingFileEntry>) => Promise<void>;
  repairingRecordingKey: string | undefined;
  requestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: Readonly<RecordingFileEntry>,
  ) => void;
  requestProcessIncompleteFiles: (channelLogin: string) => void;
  selectedIncompleteFilenames: Set<string>;
  selectedQuality: (channelLogin: string) => string;
  toggleAutoRecord: (channelLogin: string) => Promise<void>;
  toggleIncompleteMergeSelection: (filename: string) => void;
  toggleManualRecording: (channelLogin: string, quality: string, title?: string) => Promise<void>;
  toggleRecordingPin: (file: Readonly<RecordingFileEntry>) => Promise<void>;
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

export function useRecordingsController(deps: RecordingsControllerDeps): RecordingsController {
  const cachedRecordings = getCachedRecordings();
  const initialActiveRecordings = cachedRecordings?.active ?? [];
  const initialCompletedRecordings = cachedRecordings?.completed ?? [];
  const initialIncompleteRecordings = cachedRecordings?.incomplete ?? [];

  const [recordingRules, setRecordingRules] = useState<Record<string, RecordingRule | undefined>>(
    {},
  );
  const [activeRecordings, setActiveRecordings] = useState<
    Record<string, ActiveRecording | undefined>
  >(buildActiveRecordings(initialActiveRecordings));
  const [completedRecordings, setCompletedRecordings] = useState<readonly RecordingFileEntry[]>(
    initialCompletedRecordings,
  );
  const [incompleteRecordings, setIncompleteRecordings] = useState<readonly RecordingFileEntry[]>(
    initialIncompleteRecordings,
  );
  const [deletingRecordingKey, setDeletingRecordingKey] = useState<string | undefined>();
  const [pinningRecordingKey, setPinningRecordingKey] = useState<string | undefined>();
  const [mergingRecordingKey, setMergingRecordingKey] = useState<string | undefined>();
  const [selectedIncompleteFilenames, setSelectedIncompleteFilenames] = useState<Set<string>>(
    new Set(),
  );
  const [pendingJob, setPendingJob] = useState<PendingRecordingJobState | undefined>();
  const [repairingRecordingKey, setRepairingRecordingKey] = useState<string | undefined>();
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | undefined>();
  const [pendingMerge, setPendingMerge] = useState<PendingMerge | undefined>();
  const { setError } = deps;

  const selectedQuality = useCallback(
    (channelLogin: string): string => recordingRules[channelLogin]?.quality ?? '720p60',
    [recordingRules],
  );

  const loadRecordingRules = useCallback(async (): Promise<void> => {
    await loadRules((rules) => {
      setRecordingRules(rules);
    });
  }, []);

  const loadRecordingState = useCallback(async (): Promise<void> => {
    await loadState({
      setActive: (recordings) => {
        setActiveRecordings(recordings);
      },
      setCompleted: (recordings) => {
        setCompletedRecordings(recordings);
      },
      setIncomplete: (recordings) => {
        setIncompleteRecordings(recordings);
      },
      setSelected: (selection) => {
        setSelectedIncompleteFilenames(selection);
      },
    });
  }, []);

  const requestDeleteRecordingFile = useCallback(
    (bucket: 'completed' | 'incomplete', file: Readonly<RecordingFileEntry>) => {
      setPendingDelete({ bucket, file });
    },
    [],
  );

  const confirmDeleteRecordingFile = useCallback(async (): Promise<void> => {
    if (!pendingDelete) {
      return;
    }
    await executeDeleteRecording({
      clearPending: () => {
        setPendingDelete(undefined);
      },
      loadRecordingState,
      pendingDelete,
      setDeletingKey: (channelKey: string | undefined) => {
        setDeletingRecordingKey(channelKey);
      },
      setError,
    });
  }, [pendingDelete, loadRecordingState, setError]);

  const cancelDeleteRecordingFile = useCallback(() => {
    setPendingDelete(undefined);
  }, []);

  const pinCtx = useCallback(
    (): RecordingPinContext => ({
      loadRecordingState,
      setError,
      setPinningKey: (channelKey: string | undefined): void => {
        setPinningRecordingKey(channelKey);
      },
    }),
    [loadRecordingState, setError],
  );

  const toggleRecordingPin = useCallback(
    async (file: Readonly<RecordingFileEntry>): Promise<void> => {
      await toggleRecordingPinHelper({
        ctx: pinCtx(),
        file,
        pinFn: pinRecordingFile,
        unpinFn: unpinRecordingFile,
      });
    },
    [pinCtx],
  );

  const repairCtx = useCallback(
    (): RepairContext => ({
      loadRecordingState,
      setError,
      setRepairingKey: (channelKey: string | undefined): void => {
        setRepairingRecordingKey(channelKey);
      },
    }),
    [loadRecordingState, setError],
  );

  const repairRecording = useCallback(
    async (file: Readonly<RecordingFileEntry>): Promise<void> => {
      await repairRecordingHelper({
        ctx: repairCtx(),
        file,
        repairFn: async (params: { channel_login: string; filename: string }): Promise<void> => {
          await repairRecordingFile(params);
        },
      });
    },
    [repairCtx],
  );

  const toggleIncompleteMergeSelection = useCallback((filename: string): void => {
    setSelectedIncompleteFilenames((prev) => {
      const next = new Set(prev);
      if (next.has(filename)) {
        next.delete(filename);
      } else {
        next.add(filename);
      }
      return next;
    });
  }, []);

  const clearMergeSelection = useCallback(() => {
    setSelectedIncompleteFilenames(new Set());
  }, []);

  const requestProcessIncompleteFiles = useCallback(
    (channelLogin: string): void => {
      const selected = [...selectedIncompleteFilenames];
      if (selected.length === 0) {
        setError('Please select at least 1 file to process');
        return;
      }
      setPendingMerge({
        action: selected.length === 1 ? 'finalize' : 'merge',
        channelLogin,
        filenames: selected,
      });
    },
    [selectedIncompleteFilenames, setError],
  );

  const confirmProcessIncompleteFiles = useCallback(async (): Promise<void> => {
    await processIncompleteFiles(pendingMerge, {
      loadStateFn: loadRecordingState,
      setError,
      setMergingKey: (channelKey: string | undefined) => {
        setMergingRecordingKey(channelKey);
      },
      setPendingJob: (value: PendingRecordingJobState | undefined) => {
        setPendingJob(value);
      },
      setPendingMerge: (value: PendingMerge | undefined) => {
        setPendingMerge(value);
      },
    });
  }, [pendingMerge, loadRecordingState, setError]);

  const cancelProcessIncompleteFiles = useCallback(() => {
    setPendingMerge(undefined);
  }, []);

  const toggleAutoRecordCallback = useCallback(
    async (login: string): Promise<void> => {
      await toggleAutoRecord({
        channelLogin: login,
        loadRulesFn: loadRecordingRules,
        qualityFn: selectedQuality,
        rules: recordingRules,
        setError,
      });
    },
    [recordingRules, selectedQuality, loadRecordingRules, setError],
  );

  const toggleManualRecordingCallback = useCallback(
    async (login: string, quality: string, title?: string): Promise<void> => {
      await toggleManualRecording({
        active: activeRecordings,
        channelLogin: login,
        loadStateFn: loadRecordingState,
        quality,
        setError,
        title,
      });
    },
    [activeRecordings, loadRecordingState, setError],
  );

  return {
    activeRecordings,
    cancelDeleteRecordingFile,
    cancelProcessIncompleteFiles,
    clearMergeSelection,
    completedRecordings,
    confirmDeleteRecordingFile,
    confirmProcessIncompleteFiles,
    deletingRecordingKey,
    incompleteRecordings,
    loadRecordingRules,
    loadRecordingState,
    mergingRecordingKey,
    pendingDelete,
    pendingJob,
    pendingMerge,
    pinningRecordingKey,
    recordingRules,
    repairRecording,
    repairingRecordingKey,
    requestDeleteRecordingFile,
    requestProcessIncompleteFiles,
    selectedIncompleteFilenames,
    selectedQuality,
    toggleAutoRecord: toggleAutoRecordCallback,
    toggleIncompleteMergeSelection,
    toggleManualRecording: toggleManualRecordingCallback,
    toggleRecordingPin,
  };
}
