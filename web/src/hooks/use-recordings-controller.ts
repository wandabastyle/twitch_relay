import { useCallback, useState } from 'react';
import type {
  RecordingFileEntry,
  ActiveRecording,
  RecordingRule,
} from '../api-client/types';
import { getCachedRecordings } from '../api-client/recordings';
import { pinRecordingFile, repairRecordingFile, unpinRecordingFile } from '../api-client';
import { readJsError } from './errors';
import {
  buildActiveRecordings,
  loadRules,
  loadState,
  toggleAutoRecord,
  toggleManualRecording,
  handleIncompleteJobOutcome,
  pollIncompleteJob,
  repairRecordingHelper,
  startIncompleteJob,
  toggleRecordingPinHelper,
  clearPendingJobState,
  executeDeleteRecording,
  type PendingRecordingJobState,
  type PendingDelete,
  type PendingMerge,
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

const EMPTY_SELECTION = 0;
const SINGLE_SELECTION = 1;

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

const useRecordingState = (): {
  activeRecordings: Record<string, ActiveRecording | undefined>;
  completedRecordings: readonly RecordingFileEntry[];
  deletingRecordingKey: string | undefined;
  incompleteRecordings: readonly RecordingFileEntry[];
  mergingRecordingKey: string | undefined;
  pendingDelete: PendingDelete | undefined;
  pendingJob: PendingRecordingJobState | undefined;
  pendingMerge: PendingMerge | undefined;
  pinningRecordingKey: string | undefined;
  recordingRules: Record<string, RecordingRule | undefined>;
  repairingRecordingKey: string | undefined;
  selectedIncompleteFilenames: Set<string>;
  setActiveRecordings: React.Dispatch<React.SetStateAction<Record<string, ActiveRecording | undefined>>>;
  setCompletedRecordings: React.Dispatch<React.SetStateAction<readonly RecordingFileEntry[]>>;
  setDeletingRecordingKey: React.Dispatch<React.SetStateAction<string | undefined>>;
  setIncompleteRecordings: React.Dispatch<React.SetStateAction<readonly RecordingFileEntry[]>>;
  setMergingRecordingKey: React.Dispatch<React.SetStateAction<string | undefined>>;
  setPendingDelete: React.Dispatch<React.SetStateAction<PendingDelete | undefined>>;
  setPendingJob: React.Dispatch<React.SetStateAction<PendingRecordingJobState | undefined>>;
  setPendingMerge: React.Dispatch<React.SetStateAction<PendingMerge | undefined>>;
  setPinningRecordingKey: React.Dispatch<React.SetStateAction<string | undefined>>;
  setRecordingRules: React.Dispatch<React.SetStateAction<Record<string, RecordingRule | undefined>>>;
  setRepairingRecordingKey: React.Dispatch<React.SetStateAction<string | undefined>>;
  setSelectedIncompleteFilenames: React.Dispatch<React.SetStateAction<Set<string>>>;
} => {
  const cachedRecordings = getCachedRecordings();
  const [recordingRules, setRecordingRules] = useState<Record<string, RecordingRule | undefined>>(
    {},
  );
  const [activeRecordings, setActiveRecordings] = useState<
    Record<string, ActiveRecording | undefined>
  >(buildActiveRecordings(cachedRecordings?.active ?? []));
  const [completedRecordings, setCompletedRecordings] = useState<readonly RecordingFileEntry[]>(
    cachedRecordings?.completed ?? [],
  );
  const [incompleteRecordings, setIncompleteRecordings] = useState<readonly RecordingFileEntry[]>(
    cachedRecordings?.incomplete ?? [],
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

  return {
    activeRecordings,
    completedRecordings,
    deletingRecordingKey,
    incompleteRecordings,
    mergingRecordingKey,
    pendingDelete,
    pendingJob,
    pendingMerge,
    pinningRecordingKey,
    recordingRules,
    repairingRecordingKey,
    selectedIncompleteFilenames,
    setActiveRecordings,
    setCompletedRecordings,
    setDeletingRecordingKey,
    setIncompleteRecordings,
    setMergingRecordingKey,
    setPendingDelete,
    setPendingJob,
    setPendingMerge,
    setPinningRecordingKey,
    setRecordingRules,
    setRepairingRecordingKey,
    setSelectedIncompleteFilenames,
  };
};

const useRecordingActions = (
  st: ReturnType<typeof useRecordingState>,
  setError: (msg: string | null) => void,
  loadRecordingState: () => Promise<void>,
): {
  cancelDeleteRecordingFile: () => void;
  cancelProcessIncompleteFiles: () => void;
  clearMergeSelection: () => void;
  confirmDeleteRecordingFile: () => Promise<void>;
  confirmProcessIncompleteFiles: () => Promise<void>;
  repairRecording: (file: Readonly<RecordingFileEntry>) => Promise<void>;
  requestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: Readonly<RecordingFileEntry>,
  ) => void;
  requestProcessIncompleteFiles: (channelLogin: string) => void;
  toggleIncompleteMergeSelection: (filename: string) => void;
  toggleRecordingPin: (file: Readonly<RecordingFileEntry>) => Promise<void>;
} => {
  const requestDeleteRecordingFile = useCallback(
    (bucket: 'completed' | 'incomplete', file: Readonly<RecordingFileEntry>) => {
      st.setPendingDelete({ bucket, file });
    },
    [st],
  );

  const confirmDeleteRecordingFile = useCallback(async (): Promise<void> => {
    if (!st.pendingDelete) {
      return;
    }
    await executeDeleteRecording({
      clearPending: () => { st.setPendingDelete(undefined); },
      loadRecordingState,
      pendingDelete: st.pendingDelete,
      setDeletingKey: st.setDeletingRecordingKey,
      setError,
    });
  }, [st, loadRecordingState, setError]);

  const cancelDeleteRecordingFile = useCallback(() => {
    st.setPendingDelete(undefined);
  }, [st]);

  const pinCtx = useCallback(
    (): RecordingPinContext => ({
      loadRecordingState,
      setError,
      setPinningKey: st.setPinningRecordingKey,
    }),
    [loadRecordingState, setError, st],
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
      setRepairingKey: st.setRepairingRecordingKey,
    }),
    [loadRecordingState, setError, st],
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
    st.setSelectedIncompleteFilenames((prev) => {
      const next = new Set(prev);
      if (next.has(filename)) {
        next.delete(filename);
      } else {
        next.add(filename);
      }
      return next;
    });
  }, [st]);

  const clearMergeSelection = useCallback(() => {
    st.setSelectedIncompleteFilenames(new Set());
  }, [st]);

  const requestProcessIncompleteFiles = useCallback(
    (channelLogin: string): void => {
      const selected = [...st.selectedIncompleteFilenames];
      if (selected.length === EMPTY_SELECTION) {
        setError('Please select at least 1 file to process');
        return;
      }
      st.setPendingMerge({
        action: selected.length === SINGLE_SELECTION ? 'finalize' : 'merge',
        channelLogin,
        filenames: selected,
      });
    },
    [st, setError],
  );

  const confirmProcessIncompleteFiles = useCallback(async (): Promise<void> => {
    await processIncompleteFiles(st.pendingMerge, {
      loadStateFn: loadRecordingState,
      setError,
      setMergingKey: st.setMergingRecordingKey,
      setPendingJob: st.setPendingJob,
      setPendingMerge: st.setPendingMerge,
    });
  }, [st, loadRecordingState, setError]);

  const cancelProcessIncompleteFiles = useCallback(() => {
    st.setPendingMerge(undefined);
  }, [st]);

  return {
    cancelDeleteRecordingFile,
    cancelProcessIncompleteFiles,
    clearMergeSelection,
    confirmDeleteRecordingFile,
    confirmProcessIncompleteFiles,
    repairRecording,
    requestDeleteRecordingFile,
    requestProcessIncompleteFiles,
    toggleIncompleteMergeSelection,
    toggleRecordingPin,
  };
};

export const useRecordingsController = (deps: RecordingsControllerDeps): RecordingsController => {
  const st = useRecordingState();
  const { setError } = deps;

  const selectedQuality = useCallback(
    (channelLogin: string): string => st.recordingRules[channelLogin]?.quality ?? '720p60',
    [st.recordingRules],
  );

  const loadRecordingRules = useCallback(async (): Promise<void> => {
    await loadRules(st.setRecordingRules);
  }, [st]);

  const loadRecordingState = useCallback(async (): Promise<void> => {
    await loadState({
      setActive: st.setActiveRecordings,
      setCompleted: st.setCompletedRecordings,
      setIncomplete: st.setIncompleteRecordings,
      setSelected: st.setSelectedIncompleteFilenames,
    });
  }, [st]);

  const actions = useRecordingActions(st, setError, loadRecordingState);

  const toggleAutoRecordCallback = useCallback(
    async (login: string): Promise<void> => {
      await toggleAutoRecord({
        channelLogin: login,
        loadRulesFn: loadRecordingRules,
        qualityFn: selectedQuality,
        rules: st.recordingRules,
        setError,
      });
    },
    [st.recordingRules, selectedQuality, loadRecordingRules, setError],
  );

  const toggleManualRecordingCallback = useCallback(
    async (login: string, quality: string, title?: string): Promise<void> => {
      await toggleManualRecording({
        active: st.activeRecordings,
        channelLogin: login,
        loadStateFn: loadRecordingState,
        quality,
        setError,
        title,
      });
    },
    [st.activeRecordings, loadRecordingState, setError],
  );

  return {
    activeRecordings: st.activeRecordings,
    cancelDeleteRecordingFile: actions.cancelDeleteRecordingFile,
    cancelProcessIncompleteFiles: actions.cancelProcessIncompleteFiles,
    clearMergeSelection: actions.clearMergeSelection,
    completedRecordings: st.completedRecordings,
    confirmDeleteRecordingFile: actions.confirmDeleteRecordingFile,
    confirmProcessIncompleteFiles: actions.confirmProcessIncompleteFiles,
    deletingRecordingKey: st.deletingRecordingKey,
    incompleteRecordings: st.incompleteRecordings,
    loadRecordingRules,
    loadRecordingState,
    mergingRecordingKey: st.mergingRecordingKey,
    pendingDelete: st.pendingDelete,
    pendingJob: st.pendingJob,
    pendingMerge: st.pendingMerge,
    pinningRecordingKey: st.pinningRecordingKey,
    recordingRules: st.recordingRules,
    repairRecording: actions.repairRecording,
    repairingRecordingKey: st.repairingRecordingKey,
    requestDeleteRecordingFile: actions.requestDeleteRecordingFile,
    requestProcessIncompleteFiles: actions.requestProcessIncompleteFiles,
    selectedIncompleteFilenames: st.selectedIncompleteFilenames,
    selectedQuality,
    toggleAutoRecord: toggleAutoRecordCallback,
    toggleIncompleteMergeSelection: actions.toggleIncompleteMergeSelection,
    toggleManualRecording: toggleManualRecordingCallback,
    toggleRecordingPin: actions.toggleRecordingPin,
  };
};