import type { ActiveRecording, RecordingFileEntry } from './types';

const ALL_CHANNELS = 'all';
const LATEST_COUNT = 3;
const SLICE_START = 0;
const SORT_LESS = -1;
const SORT_GREATER = 1;
const SORT_EQUAL = 0;

export const recordingDeleteKey = (
  bucket: 'completed' | 'incomplete',
  file: Readonly<RecordingFileEntry>,
): string => `${bucket}:${file.channel_login}:${file.filename}`;

export const latestThree = <EntryType>(entries: readonly EntryType[]): EntryType[] =>
  entries.slice(SLICE_START, LATEST_COUNT);

export const recordingChannelOptions = (
  completedRecordings: readonly Readonly<RecordingFileEntry>[],
  incompleteRecordings: readonly Readonly<RecordingFileEntry>[],
  activeRecordings: Readonly<Record<string, Readonly<ActiveRecording> | undefined>>,
): string[] => {
  const known: Record<string, true> = {};
  for (const item of completedRecordings) {
    known[item.channel_login] = true;
  }
  for (const item of incompleteRecordings) {
    known[item.channel_login] = true;
  }
  for (const item of Object.values(activeRecordings)) {
    if (item) {
      known[item.channel_login] = true;
    }
  }
  const channels = Object.keys(known);
  return channels.toSorted((first: string, second: string) => {
    if (first < second) {
      return SORT_LESS;
    }
    if (first > second) {
      return SORT_GREATER;
    }
    return SORT_EQUAL;
  });
};

export const filterRecordingsByChannel = <EntryType extends { readonly channel_login: string }>(
  entries: readonly EntryType[],
  channelFilter: string,
): EntryType[] => {
  if (channelFilter === ALL_CHANNELS) {
    return [...entries];
  }
  return entries.filter((entry) => entry.channel_login === channelFilter);
};

export const shownRecordingEntries = <EntryType extends { readonly channel_login: string }>(
  entries: readonly EntryType[],
  channelFilter: string,
): EntryType[] => {
  const filtered = filterRecordingsByChannel(entries, channelFilter);
  if (channelFilter === ALL_CHANNELS) {
    return latestThree(filtered);
  }
  return filtered;
};
