import {
  getRecordingRules,
  getRecordings,
  startRecording,
  stopRecording,
  upsertRecordingRule,
  deleteRecordingFile,
  pinRecordingFile,
  unpinRecordingFile,
  mergeRecordingFiles,
  finalizeIncompleteRecording,
  getRecordingJobStatus,
  repairRecordingFile,
} from "$lib/api";
import { readMessage } from "$lib/home/errors";
import type {
  RecordingRule,
  ActiveRecording,
  RecordingFileEntry,
  RecordingJobKind,
  RecordingJobStatusResponse,
} from "$lib/api-client/types";

interface PendingRecordingJobState {
  jobId: string;
  kind: RecordingJobKind;
  channelLogin: string;
  expectedFilename: string;
  sourceCount: number;
  status: RecordingJobStatusResponse["status"];
}

export interface RecordingsControllerDeps {
  setError: (message: string | null) => void;
}

export interface RecordingsController {
  recordingRules: Record<string, RecordingRule>;
  activeRecordings: Record<string, ActiveRecording>;
  completedRecordings: Array<RecordingFileEntry>;
  incompleteRecordings: Array<RecordingFileEntry>;
  deletingRecordingKey: string | null;
  pinningRecordingKey: string | null;
  mergingRecordingKey: string | null;
  selectedIncompleteFilenames: Set<string>;
  pendingJob: PendingRecordingJobState | null;
  repairingRecordingKey: string | null;

  loadRecordingRules: () => Promise<void>;
  loadRecordingState: () => Promise<void>;
  toggleAutoRecord: (channelLogin: string) => Promise<void>;
  toggleManualRecording: (channelLogin: string, quality: string, title?: string) => Promise<void>;
  removeRecordingFile: (
    bucket: "completed" | "incomplete",
    file: RecordingFileEntry,
  ) => Promise<void>;
  toggleRecordingPin: (file: RecordingFileEntry) => Promise<void>;
  repairRecording: (file: RecordingFileEntry) => Promise<void>;
  toggleIncompleteMergeSelection: (filename: string) => void;
  clearMergeSelection: () => void;
  processSelectedIncompleteFiles: (channelLogin: string) => Promise<void>;
  selectedQuality: (channelLogin: string) => string;
}

export function createRecordingsController(deps: RecordingsControllerDeps): RecordingsController {
  let recordingRules = $state<Record<string, RecordingRule>>({});
  let activeRecordings = $state<Record<string, ActiveRecording>>({});
  let completedRecordings = $state<Array<RecordingFileEntry>>([]);
  let incompleteRecordings = $state<Array<RecordingFileEntry>>([]);
  let deletingRecordingKey = $state<string | null>(null);
  let pinningRecordingKey = $state<string | null>(null);
  let mergingRecordingKey = $state<string | null>(null);
  let selectedIncompleteFilenames = $state<Set<string>>(new Set());
  let pendingJob = $state<PendingRecordingJobState | null>(null);
  let repairingRecordingKey = $state<string | null>(null);

  const { setError } = deps;

  async function loadRecordingRules(): Promise<void> {
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
  }

  async function loadRecordingState(): Promise<void> {
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
  }

  function selectedQuality(channelLogin: string): string {
    return recordingRules[channelLogin]?.quality || "best";
  }

  async function toggleAutoRecord(channelLogin: string): Promise<void> {
    const current = recordingRules[channelLogin];
    const enabled = !current?.enabled;
    try {
      await upsertRecordingRule({
        channel_login: channelLogin,
        enabled,
        quality: selectedQuality(channelLogin),
        stop_when_offline: current?.stop_when_offline ?? true,
        max_duration_minutes: current?.max_duration_minutes ?? null,
        keep_last_videos: current?.keep_last_videos ?? null,
      });
      await loadRecordingRules();
    } catch (err) {
      setError(readMessage(err, "failed to toggle auto-record"));
    }
  }

  async function toggleManualRecording(
    channelLogin: string,
    quality: string,
    title?: string,
  ): Promise<void> {
    const active = activeRecordings[channelLogin];
    try {
      if (active) {
        await stopRecording(channelLogin);
      } else {
        await startRecording(channelLogin, quality, title);
      }
      await loadRecordingState();
    } catch (err) {
      setError(readMessage(err, "failed to toggle recording"));
    }
  }

  async function removeRecordingFile(
    bucket: "completed" | "incomplete",
    file: RecordingFileEntry,
  ): Promise<void> {
    const shouldDelete = window.confirm(`Delete ${file.filename}?`);
    if (!shouldDelete) {
      return;
    }

    const key = `${bucket}:${file.channel_login}:${file.filename}`;
    deletingRecordingKey = key;
    setError(null);
    try {
      await deleteRecordingFile({
        bucket,
        channel_login: file.channel_login,
        filename: file.filename,
      });
      await loadRecordingState();
    } catch (err) {
      setError(readMessage(err, "failed to delete recording"));
    } finally {
      deletingRecordingKey = null;
    }
  }

  async function toggleRecordingPin(file: RecordingFileEntry): Promise<void> {
    const key = `completed:${file.channel_login}:${file.filename}`;
    pinningRecordingKey = key;
    setError(null);

    try {
      if (file.pinned) {
        await unpinRecordingFile({
          bucket: "completed",
          channel_login: file.channel_login,
          filename: file.filename,
        });
      } else {
        await pinRecordingFile({
          bucket: "completed",
          channel_login: file.channel_login,
          filename: file.filename,
        });
      }
      await loadRecordingState();
    } catch (err) {
      setError(
        readMessage(err, file.pinned ? "failed to unpin recording" : "failed to pin recording"),
      );
    } finally {
      pinningRecordingKey = null;
    }
  }

  async function repairRecording(file: RecordingFileEntry): Promise<void> {
    const key = `completed:${file.channel_login}:${file.filename}`;
    repairingRecordingKey = key;
    setError(null);
    try {
      await repairRecordingFile({
        channel_login: file.channel_login,
        filename: file.filename,
      });
      await loadRecordingState();
    } catch (err) {
      setError(readMessage(err, "failed to repair recording"));
    } finally {
      repairingRecordingKey = null;
    }
  }

  function toggleIncompleteMergeSelection(filename: string): void {
    const newSelection = new Set(selectedIncompleteFilenames);
    if (newSelection.has(filename)) {
      newSelection.delete(filename);
    } else {
      newSelection.add(filename);
    }
    selectedIncompleteFilenames = newSelection;
  }

  function clearMergeSelection(): void {
    selectedIncompleteFilenames = new Set();
  }

  async function processSelectedIncompleteFiles(channelLogin: string): Promise<void> {
    const selectedFiles = Array.from(selectedIncompleteFilenames);
    if (selectedFiles.length === 0) {
      setError("Please select at least 1 file to process");
      return;
    }

    const action = selectedFiles.length === 1 ? "finalize" : "merge";

    const shouldContinue = window.confirm(
      `${action === "finalize" ? "Finalize" : "Merge"} ${selectedFiles.length} incomplete recording(s) for ${channelLogin}?`,
    );
    if (!shouldContinue) {
      return;
    }

    mergingRecordingKey = channelLogin;
    setError(null);

    try {
      const startResponse =
        action === "finalize"
          ? await finalizeIncompleteRecording({
              channel_login: channelLogin,
              filename: selectedFiles[0],
            })
          : await mergeRecordingFiles({
              channel_login: channelLogin,
              filenames: selectedFiles,
            });

      pendingJob = {
        jobId: startResponse.job_id,
        kind: startResponse.kind,
        channelLogin: startResponse.channel_login,
        expectedFilename: startResponse.expected_filename,
        sourceCount: startResponse.source_count,
        status: "queued",
      };

      const startedAt = Date.now();
      while (Date.now() - startedAt < 10 * 60 * 1000) {
        await new Promise((resolve) => setTimeout(resolve, 1500));
        const status = await getRecordingJobStatus(startResponse.job_id);
        pendingJob = {
          jobId: status.job_id,
          kind: status.kind,
          channelLogin: status.channel_login,
          expectedFilename: status.expected_filename,
          sourceCount: startResponse.source_count,
          status: status.status,
        };

        if (status.status === "completed") {
          pendingJob = null;
          await loadRecordingState();
          return;
        }
        if (status.status === "failed") {
          pendingJob = null;
          setError(status.error ?? "recording job failed");
          return;
        }
      }

      setError("recording job polling timed out");
    } catch (err) {
      pendingJob = null;
      setError(readMessage(err, "failed to process recordings"));
    } finally {
      mergingRecordingKey = null;
    }
  }

  return {
    get recordingRules() {
      return recordingRules;
    },
    get activeRecordings() {
      return activeRecordings;
    },
    get completedRecordings() {
      return completedRecordings;
    },
    get incompleteRecordings() {
      return incompleteRecordings;
    },
    get deletingRecordingKey() {
      return deletingRecordingKey;
    },
    get pinningRecordingKey() {
      return pinningRecordingKey;
    },
    get mergingRecordingKey() {
      return mergingRecordingKey;
    },
    get selectedIncompleteFilenames() {
      return selectedIncompleteFilenames;
    },
    get pendingJob() {
      return pendingJob;
    },
    get repairingRecordingKey() {
      return repairingRecordingKey;
    },
    loadRecordingRules,
    loadRecordingState,
    toggleAutoRecord,
    toggleManualRecording,
    removeRecordingFile,
    toggleRecordingPin,
    repairRecording,
    toggleIncompleteMergeSelection,
    clearMergeSelection,
    processSelectedIncompleteFiles,
    selectedQuality,
  };
}
