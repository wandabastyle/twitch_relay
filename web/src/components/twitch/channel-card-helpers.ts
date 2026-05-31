import type { ActiveRecording } from '../../api-client/types';

export const getRecordingTitle = (activeRecording: ActiveRecording | undefined): string => {
  if (activeRecording?.mode === 'manual') {
    return 'Stop manual recording';
  }
  if (activeRecording?.mode === 'auto') {
    return 'Stop auto recording';
  }
  return 'Start recording now';
};

export const getRecordingLabel = (activeRecording: ActiveRecording | undefined): string => {
  if (activeRecording?.mode === 'manual') {
    return 'Stop manual recording';
  }
  if (activeRecording?.mode === 'auto') {
    return 'Stop auto recording';
  }
  return 'Start recording now';
};

export const getRecordingClass = (activeRecording: ActiveRecording | undefined): string => {
  if (activeRecording?.mode === 'manual') {
    return 'active-manual';
  }
  if (activeRecording?.mode === 'auto') {
    return 'active-auto';
  }
  return '';
};
