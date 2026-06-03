export { useAuthController } from './use-auth-controller';
export { useChannelsController } from './use-channels-controller';
export { useQrController } from './use-qr-controller';
export { useRecordingsController } from './use-recordings-controller';
export type { AuthController, AuthMode } from './use-auth-controller';
export type { ChannelsController } from './use-channels-controller';
export type { QrController } from './use-qr-controller';
export type { RecordingsController } from './use-recordings-controller';
export type {
  PendingDelete,
  PendingMerge,
  PendingRecordingJobState,
} from './recordings-controller-state';

// Watch hooks
export { useVideoPlayer } from './watch/use-video-player';
export { useChat } from './watch/use-chat';
export type { UseVideoPlayerReturn } from './watch/use-video-player';
export type { UseChatReturn, ChatStatus } from './watch/use-chat';
