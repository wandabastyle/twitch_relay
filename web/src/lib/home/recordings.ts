import type { ActiveRecording, RecordingFileEntry } from '$lib/api-client';

export function latestThree<T>(entries: Array<T>): Array<T> {
  return entries.slice(0, 3);
}

export function recordingDeleteKey(
  bucket: 'completed' | 'incomplete',
  file: RecordingFileEntry,
): string {
  return `${bucket}:${file.channel_login}:${file.filename}`;
}

export function recordingChannelOptions(
  completedRecordings: Array<RecordingFileEntry>,
  incompleteRecordings: Array<RecordingFileEntry>,
  activeRecordings: Record<string, ActiveRecording>,
): Array<string> {
  const known: Record<string, true> = {};
  for (const item of completedRecordings) {
    known[item.channel_login] = true;
  }
  for (const item of incompleteRecordings) {
    known[item.channel_login] = true;
  }
  for (const item of Object.values(activeRecordings)) {
    known[item.channel_login] = true;
  }
  return Object.keys(known).sort((a, b) => a.localeCompare(b));
}

export function filterRecordingsByChannel<T extends { channel_login: string }>(
  entries: Array<T>,
  channelFilter: string,
): Array<T> {
  if (channelFilter === 'all') {
    return entries;
  }
  return entries.filter((entry) => entry.channel_login === channelFilter);
}

export function shownRecordingEntries<T extends { channel_login: string }>(
  entries: Array<T>,
  channelFilter: string,
): Array<T> {
  const filtered = filterRecordingsByChannel(entries, channelFilter);
  return channelFilter === 'all' ? latestThree(filtered) : filtered;
}
