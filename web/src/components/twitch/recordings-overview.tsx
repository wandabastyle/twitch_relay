import { useMemo, type ReactElement } from 'react';
import type { ActiveRecording, RecordingFileEntry } from '../../api-client/types';
import type { PendingMerge, PendingRecordingJobState } from '../../hooks';
import {
  filterRecordingsByChannel,
  recordingChannelOptions,
  shownRecordingEntries,
} from '../../lib/recordings';
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
    <div className="recordings-view">
      <div className="recordings-header">
        <div>
          <span className="ui-section-title">Recordings overview</span>
          <p className="recordings-subtle">Recent recording activity and files</p>
        </div>
        <button type="button" className="ui-nav-chip" onClick={onBackToChannels}>
          Back to channels
        </button>
      </div>

      <RecordingFilters
        channelOptions={channelOptions}
        recordingsChannelFilter={recordingsChannelFilter}
        onUpdateFilter={onUpdateFilter}
      />

      <LoadedFade loaded={true}>
        <div className="recordings-grid">
          {pendingJob && (
            <section className="recordings-section">
              <h2>Pending {pendingJob.kind}</h2>
              <p className="ui-muted">
                {pendingJob.channelLogin}: {pendingJob.status} ({pendingJob.sourceCount} files)
                -&gt;
                {pendingJob.expectedFilename}
              </p>
            </section>
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
        </div>
      </LoadedFade>
    </div>
  );
};
