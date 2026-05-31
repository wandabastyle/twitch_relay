import { useCallback } from 'react';
import { upsertRecordingRule } from '../../api-client';
import type { UseRecordingRuleFormReturn } from './recording-rule-form';

const FAILED_TO_SAVE = 'failed to save settings';
const MIN_MESSAGE_LENGTH = 0;

export interface SaveSettingsDeps {
  channelLogin: string;
  setIsSaving: (isSaving: boolean) => void;
  setErrorMessage: (message: string | null) => void;
  setSuccessMessage: (message: string | null) => void;
  form: Pick<
    UseRecordingRuleFormReturn,
    | 'buildSavePayload'
    | 'applyRule'
    | 'scheduleSuccessDismiss'
    | 'clearMessages'
  >;
}

export interface UseSaveSettingsReturn {
  saveSettings: (event: React.SyntheticEvent<HTMLFormElement>) => void;
}

export const useSaveSettings = (deps: SaveSettingsDeps): UseSaveSettingsReturn => {
  const { channelLogin, setIsSaving, setErrorMessage, setSuccessMessage, form } = deps;

  const readErrorMessage = useCallback((error: unknown, fallback: string): string => {
    if (error instanceof Error && error.message.trim().length > MIN_MESSAGE_LENGTH) {
      return error.message;
    }
    return fallback;
  }, []);

  const handleSaveError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readErrorMessage(error, FAILED_TO_SAVE));
    },
    [readErrorMessage, setErrorMessage],
  );

  const saveSettings = useCallback(
    (event: React.SyntheticEvent<HTMLFormElement>): void => {
      event.preventDefault();
      setIsSaving(true);
      form.clearMessages();

      void (async (): Promise<void> => {
        try {
          const saved = await upsertRecordingRule(form.buildSavePayload(channelLogin));

          form.applyRule(saved);
          setSuccessMessage('Saved');
          form.scheduleSuccessDismiss();
        } catch (error) {
          handleSaveError(error);
        } finally {
          setIsSaving(false);
        }
      })();
    },
    [channelLogin, setIsSaving, form, handleSaveError, setSuccessMessage],
  );

  return { saveSettings };
};
