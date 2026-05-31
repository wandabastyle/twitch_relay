import type {
  YouTubeVideoMeta,
  YouTubeWatchProgress,
  YouTubeEmbedConfig,
} from '../../api-client/types';
import {
  getYouTubeEmbedConfig,
  getYouTubeVideoMeta,
  getYouTubeVideoProgress,
} from '../../api-client/youtube-progress';
import { getEndGapSecs } from './time-utils';

const ZERO = 0;
const RESUME_MIN_SECS = 15;
const DEFAULT_REFERRER_POLICY: 'no-referrer' | 'strict-origin-when-cross-origin' =
  'no-referrer';

export interface BuildEmbedUrlOptions {
  videoId: string;
  defaults: { autoplay: number; quality: string; quality_dash: string };
  resumeAtSecs: number | null;
}

export const buildEmbedUrl = (options: BuildEmbedUrlOptions): string => {
  const { videoId, defaults, resumeAtSecs } = options;
  const params = new URLSearchParams({
    autoplay: String(defaults.autoplay),
    quality: defaults.quality,
    quality_dash: defaults.quality_dash,
  });

  if (resumeAtSecs !== null && resumeAtSecs >= RESUME_MIN_SECS) {
    const resumeSeconds = String(Math.floor(resumeAtSecs));
    params.set('start', resumeSeconds);
    params.set('t', `${resumeSeconds}s`);
  }
  return `/api/youtube/embed/${encodeURIComponent(videoId)}?${params.toString()}`;
};

export const determineResumePosition = (
  progress: YouTubeWatchProgress,
  duration: number,
  endGap: number,
): number | null => {
  const positionSecs = progress.position_secs;
  const maxPosition = Math.max(ZERO, duration - endGap);
  if (
    !progress.completed &&
    positionSecs !== null &&
    positionSecs >= RESUME_MIN_SECS &&
    positionSecs <= maxPosition
  ) {
    return positionSecs;
  }
  return null;
};

export interface VideoInitData {
  embedConfig: YouTubeEmbedConfig;
  videoMeta: YouTubeVideoMeta;
  savedProgress: YouTubeWatchProgress;
}

export const loadVideoInitData = async (
  videoId: string,
): Promise<VideoInitData> => {
  const [embedConfig, videoMeta, savedProgress] = await Promise.all([
    getYouTubeEmbedConfig(),
    getYouTubeVideoMeta(videoId),
    getYouTubeVideoProgress(videoId),
  ]);
  return { embedConfig, savedProgress, videoMeta };
};

export const getReferrerPolicy = (
  embedConfig: YouTubeEmbedConfig,
): 'no-referrer' | 'strict-origin-when-cross-origin' =>
  embedConfig.referrer_policy === 'strict-origin-when-cross-origin'
    ? 'strict-origin-when-cross-origin'
    : DEFAULT_REFERRER_POLICY;

export interface VideoInitResult {
  embedUrl: string;
  referrerPolicy: 'no-referrer' | 'strict-origin-when-cross-origin';
  resumeAt: number | null;
  videoDuration: number;
  videoTitle: string;
}

export const initializeVideoData = async (
  videoId: string,
): Promise<VideoInitResult> => {
  const { embedConfig, videoMeta, savedProgress } = await loadVideoInitData(videoId);
  const endGap = getEndGapSecs(videoMeta.duration);
  const resumeAt = determineResumePosition(savedProgress, videoMeta.duration, endGap);
  const embedUrl = buildEmbedUrl({
    defaults: embedConfig.defaults,
    resumeAtSecs: resumeAt,
    videoId,
  });
  const referrerPolicy = getReferrerPolicy(embedConfig);

  return {
    embedUrl,
    referrerPolicy,
    resumeAt,
    videoDuration: videoMeta.duration,
    videoTitle: videoMeta.title,
  };
};
