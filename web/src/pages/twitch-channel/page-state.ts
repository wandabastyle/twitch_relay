import { useCallback, useState } from 'react';
import { getChannels, getRecordingRules } from '../../api-client';
import { useRecordingRuleForm } from './recording-rule-form';
import { useSaveSettings } from './save-settings';
import { readErrorMessage } from './validation';

const FAILED_TO_LOAD = 'failed to load channel settings';

export interface PageStateDeps {
  channelLogin: string;
}

export interface PageState {
  channelDisplayName: string;
  channelExists: boolean;
  isLoading: boolean;
  isSaving: boolean;
}

export interface PageActions {
  loadPageState: () => Promise<void>;
}

export interface UseChannelPageStateReturn {
  channelDisplayName: string;
  channelExists: boolean;
  form: ReturnType<typeof useRecordingRuleForm>;
  isLoading: boolean;
  isSaving: boolean;
  loadPageState: () => Promise<void>;
  saveSettings: (event: React.SyntheticEvent<HTMLFormElement>) => void;
}

export const useChannelPageState = (deps: PageStateDeps): UseChannelPageStateReturn => {
  const { channelLogin } = deps;

  const [channelExists, setChannelExists] = useState(true);
  const [channelDisplayName, setChannelDisplayName] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  const form = useRecordingRuleForm();
  const { applyRule, clearMessages, setErrorMessage } = form;

  const handleLoadError = useCallback(
    (error: unknown): void => {
      setErrorMessage(readErrorMessage(error, FAILED_TO_LOAD));
    },
    [setErrorMessage],
  );

  const loadChannels = useCallback(async (): Promise<void> => {
    const [channels, rules] = await Promise.all([getChannels(), getRecordingRules()]);
    const channel = channels.find((entry) => entry.login === channelLogin);
    setChannelExists(Boolean(channel));
    setChannelDisplayName(channel?.display_name ?? channel?.login ?? channelLogin);

    const rule = rules.find((entry) => entry.channel_login === channelLogin);
    applyRule(rule ?? undefined);
  }, [channelLogin, applyRule]);

  const loadPageState = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    clearMessages();

    try {
      await loadChannels();
    } catch (error) {
      handleLoadError(error);
    } finally {
      setIsLoading(false);
    }
  }, [clearMessages, loadChannels, handleLoadError]);

  const { saveSettings } = useSaveSettings({
    channelLogin,
    form,
    setErrorMessage,
    setIsSaving,
    setSuccessMessage: form.setSuccessMessage,
  });

  return {
    channelDisplayName,
    channelExists,
    form,
    isLoading,
    isSaving,
    loadPageState,
    saveSettings,
  };
};
