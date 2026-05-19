import type { ActiveRecording, RecordingFileEntry } from '$lib/api-client';

const ALL_CHANNELS = 'all';
const LATEST_COUNT = 3;
const SLICE_START = 0;

export const recordingDeleteKey = (
  bucket: 'completed' | 'incomplete',
  file: RecordingFileEntry,
): string => `${bucket}:${file.channel_login}:${file.filename}`;

export const latestThree = <EntryType>(entries: EntryType[]): EntryType[] =>
  entries.slice(SLICE_START, LATEST_COUNT);

export const recordingChannelOptions = (
  completedRecordings: RecordingFileEntry[],
  incompleteRecordings: RecordingFileEntry[],
  activeRecordings: Record<string, ActiveRecording>,
): string[] => {
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
  return Object.keys(known)
    .slice()
    .sort((first, second) => first.localeCompare(second));
};

export const filterRecordingsByChannel = <EntryType extends { channel_login: string }>(
  entries: EntryType[],
  channelFilter: string,
): EntryType[] => {
  if (channelFilter === ALL_CHANNELS) {
    return entries;
  }
  return entries.filter((entry) => entry.channel_login === channelFilter);
};

export const shownRecordingEntries = <EntryType extends { channel_login: string }>(
  entries: EntryType[],
  channelFilter: string,
): EntryType[] => {
  const filtered = filterRecordingsByChannel(entries, channelFilter);
  if (channelFilter === ALL_CHANNELS) {
    return latestThree(filtered);
  }
  return filtered;
};
