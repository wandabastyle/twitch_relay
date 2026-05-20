import type {
  ActiveRecording,
  ChannelEntry,
  ChannelStatus,
  RecordingFileEntry,
  RecordingRule,
  TwitchStatusResponse,
} from '$lib/api-client/types';

// Re-export types for use in components
export type { ChannelEntry, ChannelStatus, RecordingRule, ActiveRecording, RecordingFileEntry };

export type AuthMode = 'checking' | 'authenticated' | 'unauthenticated';
export type RelayMode = 'twitch' | 'youtube';
export type LoginMode = 'code' | 'qr';

type SubmitHandler = {
  bivarianceHack(event: unknown): void;
}['bivarianceHack'];

// AppHeader props
export interface AppHeaderProps {
  authMode: AuthMode;
  relayMode: RelayMode;
  twitchStatus: TwitchStatusResponse;
  isTwitchStatusLoaded: boolean;
  isTwitchBusy: boolean;
  isBusy: boolean;
  onToggleMode: () => void;
  onConnectTwitch: () => void;
  onDisconnectTwitch: () => void;
  onSignOut: () => void;
}

// AuthPanel props
export interface AuthPanelProps {
  loginMode: LoginMode;
  accessCode: string;
  qrDataUrl: string | undefined;
  isBusy: boolean;
  onSubmitLogin: SubmitHandler;
  onSwitchToQr: () => void;
  onSwitchToCode: () => void;
  onUpdateAccessCode: (value: string) => void;
}

// ChannelCard props
export interface ChannelCardProps {
  channel: ChannelEntry;
  status: ChannelStatus | undefined;
  recordingRule: RecordingRule | undefined;
  activeRecording: ActiveRecording | undefined;
  isWatching: boolean;
  onOpenSetup: () => void;
  onStartWatching: () => void;
  onToggleAutoRecord: () => void;
  onToggleManualRecording: () => void;
  onRemove: () => void;
}

// AddChannelForm props
export interface AddChannelFormProps {
  newChannelLogin: string;
  isAdding: boolean;
  onSubmit: SubmitHandler;
  onCancel: () => void;
  onUpdateValue: (value: string) => void;
}

// TwitchChannelsView props
export interface TwitchChannelsViewProps {
  channels: ChannelEntry[];
  liveStatus: Record<string, ChannelStatus>;
  liveOnly?: boolean;
  showAddForm: boolean;
  newChannelLogin: string;
  isAddingChannel: boolean;
  watchingChannel: string | undefined;
  recordingRules: Record<string, RecordingRule | undefined>;
  activeRecordings: Record<string, ActiveRecording | undefined>;
  liveStatusError: string | undefined;
  isLiveStatusLoaded: boolean;
  onLiveOnlyChange: (value: boolean) => void;
  onOpenRecordings: () => void;
  onShowAddForm: () => void;
  onCancelAddForm: () => void;
  onSubmitAddChannel: SubmitHandler;
  onUpdateNewChannelLogin: (value: string) => void;
  onOpenChannelSetup: (login: string) => void;
  onStartWatching: (login: string) => void;
  onToggleAutoRecord: (login: string) => void;
  onToggleManualRecording: (login: string) => void;
  onPromptRemoveChannel: (login: string) => void;
}

// RecordingsOverview props
export interface RecordingsOverviewProps {
  activeRecordings: Record<string, ActiveRecording | undefined>;
  completedRecordings: readonly RecordingFileEntry[];
  incompleteRecordings: readonly RecordingFileEntry[];
  recordingsChannelFilter: string;
  deletingRecordingKey: string | undefined;
  pinningRecordingKey: string | undefined;
  repairingRecordingKey: string | undefined;
  mergingRecordingKey: string | undefined;
  selectedIncompleteFilenames: ReadonlySet<string>;
  pendingJob:
    | {
        jobId: string;
        kind: 'merge' | 'finalize';
        channelLogin: string;
        expectedFilename: string;
        sourceCount: number;
        status: 'queued' | 'running' | 'completed' | 'failed';
      }
    | undefined;
  pendingDelete: { bucket: 'completed' | 'incomplete'; file: RecordingFileEntry } | undefined;
  pendingMerge:
    | { channelLogin: string; action: 'finalize' | 'merge'; filenames: readonly string[] }
    | undefined;
  onBackToChannels: () => void;
  onUpdateFilter: (value: string) => void;
  onOpenRecordingPlayer: (file: Readonly<RecordingFileEntry>) => void;
  onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: Readonly<RecordingFileEntry>,
  ) => void;
  onConfirmDeleteRecordingFile: () => void;
  onCancelDeleteRecordingFile: () => void;
  onToggleRecordingPin: (file: Readonly<RecordingFileEntry>) => void;
  onRepairRecording: (file: Readonly<RecordingFileEntry>) => void;
  onToggleIncompleteMergeSelection: (filename: string) => void;
  onRequestProcessIncompleteFiles: (channelLogin: string) => void;
  onConfirmProcessIncompleteFiles: () => void;
  onCancelProcessIncompleteFiles: () => void;
}
