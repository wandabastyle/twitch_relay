import {
  getRecordingRules,
  getRecordings,
  startRecording,
  stopRecording,
  upsertRecordingRule,
  deleteRecordingFile,
  pinRecordingFile,
  unpinRecordingFile,
} from "$lib/api";
import { readMessage } from "$lib/home/errors";
import type { RecordingRule, ActiveRecording, RecordingFileEntry } from "$lib/api-client/types";

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

  loadRecordingRules: () => Promise<void>;
  loadRecordingState: () => Promise<void>;
  toggleAutoRecord: (channelLogin: string) => Promise<void>;
  toggleManualRecording: (channelLogin: string, quality: string, title?: string) => Promise<void>;
  removeRecordingFile: (
    bucket: "completed" | "incomplete",
    file: RecordingFileEntry,
  ) => Promise<void>;
  toggleRecordingPin: (file: RecordingFileEntry) => Promise<void>;
  selectedQuality: (channelLogin: string) => string;
}

export function createRecordingsController(deps: RecordingsControllerDeps): RecordingsController {
  let recordingRules = $state<Record<string, RecordingRule>>({});
  let activeRecordings = $state<Record<string, ActiveRecording>>({});
  let completedRecordings = $state<Array<RecordingFileEntry>>([]);
  let incompleteRecordings = $state<Array<RecordingFileEntry>>([]);
  let deletingRecordingKey = $state<string | null>(null);
  let pinningRecordingKey = $state<string | null>(null);

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
    loadRecordingRules,
    loadRecordingState,
    toggleAutoRecord,
    toggleManualRecording,
    removeRecordingFile,
    toggleRecordingPin,
    selectedQuality,
  };
}
