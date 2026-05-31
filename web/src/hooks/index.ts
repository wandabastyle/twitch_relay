export { useAuthController } from './useAuthController';
export { useChannelsController } from './useChannelsController';
export { useQrController } from './useQrController';
export { useRecordingsController } from './useRecordingsController';
export type { AuthController, AuthMode } from './useAuthController';
export type { ChannelsController } from './useChannelsController';
export type { QrController } from './useQrController';
export type { RecordingsController } from './useRecordingsController';
export type {
  PendingDelete,
  PendingMerge,
  PendingRecordingJobState,
} from './recordings-controller-state';

// Watch hooks
export { useVideoPlayer } from './watch/useVideoPlayer';
export { useChat } from './watch/useChat';
export { useChatComposer } from './watch/useChatComposer';
export type { UseVideoPlayerReturn } from './watch/useVideoPlayer';
export type { UseChatReturn, ChatStatus } from './watch/useChat';
export type { UseChatComposerReturn } from './watch/useChatComposer';
