import type {
  ChannelEntry,
  ChannelStatus,
  RecordingRule,
  ActiveRecording,
  RecordingFileEntry,
  TwitchStatusResponse,
} from "$lib/api-client/types";

// Re-export types for use in components
export type { ChannelEntry, ChannelStatus, RecordingRule, ActiveRecording, RecordingFileEntry };

export type AuthMode = "checking" | "authenticated" | "unauthenticated";
export type RelayMode = "twitch" | "youtube";
export type LoginMode = "code" | "qr";
export type YouTubeViewMode = "subscriptions" | "recent" | "playlists";
export type CurrentView = "channels" | "recordings";

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
  qrDataUrl: string | null;
  isBusy: boolean;
  onSubmitLogin: (event: SubmitEvent) => void;
  onSwitchToQr: () => void;
  onSwitchToCode: () => void;
  onUpdateAccessCode: (value: string) => void;
}

// YouTubeModeView props
export interface YouTubeModeViewProps {
  youtubeViewMode: YouTubeViewMode;
  onViewModeChange: (mode: YouTubeViewMode) => void;
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
  onSubmit: (event: SubmitEvent) => void;
  onCancel: () => void;
  onUpdateValue: (value: string) => void;
}

// TwitchChannelsView props
export interface TwitchChannelsViewProps {
  channels: ChannelEntry[];
  liveStatus: Record<string, ChannelStatus>;
  liveOnly: boolean;
  showAddForm: boolean;
  newChannelLogin: string;
  isAddingChannel: boolean;
  watchingChannel: string | null;
  recordingRules: Record<string, RecordingRule>;
  activeRecordings: Record<string, ActiveRecording>;
  liveStatusError: string | null;
  onLiveOnlyChange: (value: boolean) => void;
  onOpenRecordings: () => void;
  onShowAddForm: () => void;
  onCancelAddForm: () => void;
  onSubmitAddChannel: (event: SubmitEvent) => void;
  onUpdateNewChannelLogin: (value: string) => void;
  onOpenChannelSetup: (login: string) => void;
  onStartWatching: (login: string) => void;
  onToggleAutoRecord: (login: string) => void;
  onToggleManualRecording: (login: string) => void;
  onPromptRemoveChannel: (login: string) => void;
}

// RecordingsOverview props
export interface RecordingsOverviewProps {
  activeRecordings: Record<string, ActiveRecording>;
  completedRecordings: RecordingFileEntry[];
  incompleteRecordings: RecordingFileEntry[];
  recordingsChannelFilter: string;
  deletingRecordingKey: string | null;
  pinningRecordingKey: string | null;
  repairingRecordingKey: string | null;
  mergingRecordingKey: string | null;
  selectedIncompleteFilenames: Set<string>;
  pendingJob: {
    jobId: string;
    kind: "merge" | "finalize";
    channelLogin: string;
    expectedFilename: string;
    sourceCount: number;
    status: "queued" | "running" | "completed" | "failed";
  } | null;
  onBackToChannels: () => void;
  onUpdateFilter: (value: string) => void;
  onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
  onRemoveRecordingFile: (bucket: "completed" | "incomplete", file: RecordingFileEntry) => void;
  onToggleRecordingPin: (file: RecordingFileEntry) => void;
  onRepairRecording: (file: RecordingFileEntry) => void;
  onToggleIncompleteMergeSelection: (filename: string) => void;
  onProcessIncompleteFiles: (channelLogin: string) => void;
}

// ConfirmRemoveDialog props
export interface ConfirmRemoveDialogProps {
  channelLogin: string | null;
  isRemoving: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}
