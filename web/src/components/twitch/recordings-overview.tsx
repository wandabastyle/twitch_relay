import { Box, Stack, Typography, Button } from '@mui/material';
import { useMemo, type ReactElement } from 'react';
import {
  filterRecordingsByChannel,
  recordingChannelOptions,
  shownRecordingEntries,
} from '../../api-client/recordings-helpers';
import type { ActiveRecording, RecordingFileEntry } from '../../api-client/types';
import type { PendingMerge, PendingRecordingJobState } from '../../hooks';
import { LoadedFade } from '../ui/loaded-fade';
import { ActiveRecordingsSection } from './active-recording-section';
import { CompletedRecordingsSection } from './completed-recordings-section';
import { IncompleteRecordingsSection } from './incomplete-recordings-section';
import { RecordingFilters } from './recording-filters';

interface RecordingsOverviewProps {
  readonly activeRecordings: Record<string, ActiveRecording | undefined>;
  readonly completedRecordings: readonly RecordingFileEntry[];
  readonly incompleteRecordings: readonly RecordingFileEntry[];
  readonly recordingsChannelFilter: string;
  readonly deletingRecordingKey: string | undefined;
  readonly pinningRecordingKey: string | undefined;
  readonly repairingRecordingKey: string | undefined;
  readonly mergingRecordingKey: string | undefined;
  readonly selectedIncompleteFilenames: Set<string>;
  readonly pendingJob: PendingRecordingJobState | undefined;
  readonly pendingDelete:
    | { bucket: 'completed' | 'incomplete'; file: RecordingFileEntry }
    | undefined;
  readonly pendingMerge: PendingMerge | undefined;
  readonly onBackToChannels: () => void;
  readonly onUpdateFilter: (value: string) => void;
  readonly onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
  readonly onToggleRecordingPin: (file: RecordingFileEntry) => void;
  readonly onRepairRecording: (file: RecordingFileEntry) => void;
  readonly onToggleIncompleteMergeSelection: (filename: string) => void;
  readonly onRequestProcessIncompleteFiles: (channelLogin: string) => void;
  readonly onConfirmDeleteRecordingFile: () => Promise<void>;
  readonly onCancelDeleteRecordingFile: () => void;
  readonly onConfirmProcessIncompleteFiles: () => Promise<void>;
  readonly onCancelProcessIncompleteFiles: () => void;
}

const EMPTY_LENGTH = 0;

export const RecordingsOverview = ({
  activeRecordings,
  completedRecordings,
  incompleteRecordings,
  recordingsChannelFilter,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  mergingRecordingKey,
  selectedIncompleteFilenames,
  pendingJob,
  onBackToChannels,
  onUpdateFilter,
  onOpenRecordingPlayer,
  onRequestDeleteRecordingFile,
  onToggleRecordingPin,
  onRepairRecording,
  onToggleIncompleteMergeSelection,
  onRequestProcessIncompleteFiles,
}: RecordingsOverviewProps): ReactElement => {
  const channelOptions = useMemo(
    () => recordingChannelOptions(completedRecordings, incompleteRecordings, activeRecordings),
    [completedRecordings, incompleteRecordings, activeRecordings],
  );

  const activeRecordingsList = useMemo(
    () =>
      Object.values(activeRecordings).filter(
        (recording): recording is ActiveRecording => recording !== undefined,
      ),
    [activeRecordings],
  );

  const activeList = useMemo(
    () => filterRecordingsByChannel(activeRecordingsList, recordingsChannelFilter),
    [activeRecordingsList, recordingsChannelFilter],
  );

  const completedList = useMemo(
    () => filterRecordingsByChannel(completedRecordings, recordingsChannelFilter),
    [completedRecordings, recordingsChannelFilter],
  );

  const incompleteList = useMemo(
    () => filterRecordingsByChannel(incompleteRecordings, recordingsChannelFilter),
    [incompleteRecordings, recordingsChannelFilter],
  );

  const shownActive = useMemo(
    () => shownRecordingEntries(activeRecordingsList, recordingsChannelFilter),
    [activeRecordingsList, recordingsChannelFilter],
  );

  const shownCompleted = useMemo(
    () => shownRecordingEntries(completedRecordings, recordingsChannelFilter),
    [completedRecordings, recordingsChannelFilter],
  );

  const shownIncomplete = useMemo(
    () => shownRecordingEntries(incompleteRecordings, recordingsChannelFilter),
    [incompleteRecordings, recordingsChannelFilter],
  );

  const selectedCount = useMemo(() => {
    if (recordingsChannelFilter !== 'all' && shownIncomplete.length > EMPTY_LENGTH) {
      return [...selectedIncompleteFilenames].filter((filename) =>
        shownIncomplete.some((file) => file.filename === filename),
      ).length;
    }
    return EMPTY_LENGTH;
  }, [recordingsChannelFilter, shownIncomplete, selectedIncompleteFilenames]);

  return (
    <Box sx={{ padding: 2 }}>
      <Stack
        direction="row"
        spacing={2}
        sx={{ alignItems: 'center', justifyContent: 'space-between', marginBottom: 2 }}
      >
        <Box>
          <Typography variant="h5" component="div" gutterBottom>
            Recordings overview
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Recent recording activity and files
          </Typography>
        </Box>
        <Button variant="text" onClick={onBackToChannels}>
          Back to channels
        </Button>
      </Stack>

      <RecordingFilters
        channelOptions={channelOptions}
        recordingsChannelFilter={recordingsChannelFilter}
        onUpdateFilter={onUpdateFilter}
      />

      <LoadedFade loaded={true}>
        <Box sx={{ display: 'grid', gap: 2 }}>
          {pendingJob && (
            <Box
              component="section"
              sx={{ backgroundColor: 'background.paper', borderRadius: 1, padding: 2 }}
            >
              <Typography variant="h6" gutterBottom>
                Pending {pendingJob.kind}
              </Typography>
              <Typography variant="body2" color="text.secondary">
                {pendingJob.channelLogin}: {pendingJob.status} ({pendingJob.sourceCount} files)
                -&gt; {pendingJob.expectedFilename}
              </Typography>
            </Box>
          )}

          <ActiveRecordingsSection activeList={activeList} shownActive={shownActive} />

          <CompletedRecordingsSection
            completedList={completedList}
            shownCompleted={shownCompleted}
            deletingRecordingKey={deletingRecordingKey}
            pinningRecordingKey={pinningRecordingKey}
            repairingRecordingKey={repairingRecordingKey}
            onToggleRecordingPin={onToggleRecordingPin}
            onOpenRecordingPlayer={onOpenRecordingPlayer}
            onRepairRecording={onRepairRecording}
            onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
          />

          <IncompleteRecordingsSection
            incompleteList={incompleteList}
            shownIncomplete={shownIncomplete}
            recordingsChannelFilter={recordingsChannelFilter}
            selectedCount={selectedCount}
            mergingRecordingKey={mergingRecordingKey}
            deletingRecordingKey={deletingRecordingKey}
            selectedIncompleteFilenames={selectedIncompleteFilenames}
            onToggleIncompleteMergeSelection={onToggleIncompleteMergeSelection}
            onRequestProcessIncompleteFiles={onRequestProcessIncompleteFiles}
            onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
          />
        </Box>
      </LoadedFade>
    </Box>
  );
};
