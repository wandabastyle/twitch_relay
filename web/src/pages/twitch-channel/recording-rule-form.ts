import { useCallback, useRef, useState } from 'react';
import type { RecordingRule } from '../../api-client';
import { parseOptionalPositiveInt } from './validation';

const DEFAULT_QUALITY = '720p60';
const SUCCESS_DISMISS_MS = 3500;

export interface FormState {
  enabled: boolean;
  quality: string;
  stopWhenOffline: boolean;
  maxDurationMinutesInput: string;
  keepLastVideosInput: string;
}

export interface MessageState {
  errorMessage: string | null;
  successMessage: string | null;
}

export const useRecordingRuleForm = () => {
  const [enabled, setEnabled] = useState(false);
  const [quality, setQuality] = useState(DEFAULT_QUALITY);
  const [stopWhenOffline, setStopWhenOffline] = useState(true);
  const [maxDurationMinutesInput, setMaxDurationMinutesInput] = useState('');
  const [keepLastVideosInput, setKeepLastVideosInput] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const successDismissTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetFormState = useCallback((): void => {
    setEnabled(false);
    setQuality(DEFAULT_QUALITY);
    setStopWhenOffline(true);
    setMaxDurationMinutesInput('');
    setKeepLastVideosInput('');
  }, []);

  const applyRuleValues = useCallback((rule: RecordingRule): void => {
    setEnabled(rule.enabled);
    setQuality(rule.quality !== undefined ? rule.quality : DEFAULT_QUALITY);
    setStopWhenOffline(rule.stop_when_offline);
    setMaxDurationMinutesInput(
      rule.max_duration_minutes === null ? '' : String(rule.max_duration_minutes),
    );
    setKeepLastVideosInput(
      rule.keep_last_videos === null ? '' : String(rule.keep_last_videos),
    );
  }, []);

  const applyRule = useCallback(
    (rule: RecordingRule | undefined): void => {
      if (!rule) {
        resetFormState();
        return;
      }
      applyRuleValues(rule);
    },
    [applyRuleValues, resetFormState],
  );

  const clearMessages = useCallback((): void => {
    setErrorMessage(null);
    setSuccessMessage(null);
  }, []);

  const scheduleSuccessDismiss = useCallback((): void => {
    if (successDismissTimerRef.current) {
      clearTimeout(successDismissTimerRef.current);
    }
    successDismissTimerRef.current = setTimeout(() => {
      setSuccessMessage(null);
    }, SUCCESS_DISMISS_MS);
  }, []);

  const parseMaxDuration = useCallback(
    (): number | undefined =>
      parseOptionalPositiveInt(maxDurationMinutesInput, 'Max duration minutes'),
    [maxDurationMinutesInput],
  );

  const parseKeepVideos = useCallback(
    (): number | undefined => parseOptionalPositiveInt(keepLastVideosInput, 'Keep last videos'),
    [keepLastVideosInput],
  );

  const buildSavePayload = useCallback(
    (channelLogin: string): {
      channel_login: string;
      enabled: boolean;
      keep_last_videos: number | undefined;
      max_duration_minutes: number | undefined;
      quality: string;
      stop_when_offline: boolean;
    } => {
      const keepVideos = parseKeepVideos();
      const maxDuration = parseMaxDuration();
      return {
        channel_login: channelLogin,
        enabled,
        keep_last_videos: keepVideos,
        max_duration_minutes: maxDuration,
        quality,
        stop_when_offline: stopWhenOffline,
      };
    },
    [enabled, quality, stopWhenOffline, parseKeepVideos, parseMaxDuration],
  );

  const cleanupTimer = useCallback((): void => {
    if (successDismissTimerRef.current) {
      clearTimeout(successDismissTimerRef.current);
    }
  }, []);

  return {
    // State
    enabled,
    quality,
    stopWhenOffline,
    maxDurationMinutesInput,
    keepLastVideosInput,
    errorMessage,
    successMessage,
    // Setters
    setEnabled,
    setQuality,
    setStopWhenOffline,
    setMaxDurationMinutesInput,
    setKeepLastVideosInput,
    setErrorMessage,
    setSuccessMessage,
    // Actions
    resetFormState,
    applyRuleValues,
    applyRule,
    clearMessages,
    scheduleSuccessDismiss,
    buildSavePayload,
    cleanupTimer,
  };
};
