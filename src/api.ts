import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettings,
  Preset,
  AudioDeviceInfo,
  MediaToolInfo,
  AudioMetadata,
  AudioProcessingResult,
  VideoMetadata,
  PlayerState,
  LiveAudioStatus,
  DspConfig,
} from './types';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const defaultDspConfig: DspConfig = {
  enabled: true,
  master_volume: 1.0,
  bass_gain_db: 0.0,
  treble_gain_db: 0.0,
  vocal_clarity_db: 0.0,
  eq_bands_db: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  spatial_enabled: false,
  spatial_width: 0.3,
  limiter_enabled: true,
  limiter_threshold_db: -0.5,
};

export const defaultSettings: AppSettings = {
  theme: 'light',
  last_page: 'sound',
  active_preset_id: 'balanced',
  current_config: defaultDspConfig,
  custom_presets: [],
  selected_device_id: undefined,
  device_profiles: {},
  last_export_dir: undefined,
  minimize_to_tray: true,
  start_with_windows: false,
  a_b_loudness_matched: true,
};

// Mock presets for browser fallback
const mockPresets: Preset[] = [
  { id: 'balanced', name: 'Balanced', description: 'Neutral, transparent response with subtle clarity enhancement.', is_builtin: true, config: { ...defaultDspConfig } },
  { id: 'music', name: 'Music', description: 'Enriched low-end warmth and smooth high-frequency sparkle for musicality.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 2.5, treble_gain_db: 2.0, vocal_clarity_db: 1.0, spatial_enabled: true, spatial_width: 0.25 } },
  { id: 'bass-boost', name: 'Bass Boost', description: 'Pronounced sub-bass and mid-bass impact protected by soft limiting.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 5.5, treble_gain_db: 0.5, limiter_threshold_db: -0.8 } },
  { id: 'vocal-clarity', name: 'Vocal Clarity', description: 'Boosts presence range (2-4 kHz) and gently reduces low-mid boominess.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: -1.5, treble_gain_db: 2.0, vocal_clarity_db: 4.5 } },
  { id: 'movies', name: 'Movies', description: 'Cinematic dynamic range shaping, vocal dialogue clarity, and virtual width.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 3.5, treble_gain_db: 1.5, vocal_clarity_db: 3.0, spatial_enabled: true, spatial_width: 0.45 } },
  { id: 'dialogue', name: 'Dialogue', description: 'Focused speech intelligibility for podcasts, interviews, and audiobooks.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: -3.0, treble_gain_db: 1.0, vocal_clarity_db: 5.0 } },
  { id: 'gaming', name: 'Gaming', description: 'Footstep and positional awareness with enhanced transient detail and spatial cues.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 1.5, treble_gain_db: 3.0, vocal_clarity_db: 2.5, spatial_enabled: true, spatial_width: 0.5 } },
  { id: 'headphones', name: 'Headphones', description: 'Compensates for in-head localization with subtle binaural crossfeed.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 1.5, treble_gain_db: 1.0, vocal_clarity_db: 1.0, spatial_enabled: true, spatial_width: 0.35 } },
  { id: 'speakers', name: 'Speakers', description: 'Tightened low-end and crisp top-end tailored for desktop monitors and speakers.', is_builtin: true, config: { ...defaultDspConfig, bass_gain_db: 1.0, treble_gain_db: 1.5, vocal_clarity_db: 1.5 } },
];

export async function apiGetSettings(): Promise<AppSettings> {
  if (isTauri) {
    return await invoke<AppSettings>('get_app_settings');
  }
  const local = localStorage.getItem('sonatune_settings');
  return local ? JSON.parse(local) : defaultSettings;
}

export async function apiSaveSettings(settings: AppSettings): Promise<AppSettings> {
  if (isTauri) {
    return await invoke<AppSettings>('save_app_settings', { newSettings: settings });
  }
  localStorage.setItem('sonatune_settings', JSON.stringify(settings));
  return settings;
}

export async function apiResetSettings(): Promise<AppSettings> {
  if (isTauri) {
    return await invoke<AppSettings>('reset_app_settings');
  }
  localStorage.removeItem('sonatune_settings');
  return defaultSettings;
}

export async function apiGetPresets(): Promise<Preset[]> {
  if (isTauri) {
    return await invoke<Preset[]>('get_presets');
  }
  const local = localStorage.getItem('sonatune_custom_presets');
  const custom = local ? JSON.parse(local) : [];
  return [...mockPresets, ...custom];
}

export async function apiSaveCustomPreset(preset: Preset): Promise<Preset[]> {
  if (isTauri) {
    return await invoke<Preset[]>('save_custom_preset', { preset });
  }
  const local = localStorage.getItem('sonatune_custom_presets');
  let custom: Preset[] = local ? JSON.parse(local) : [];
  custom = custom.filter(p => p.id !== preset.id);
  custom.push(preset);
  localStorage.setItem('sonatune_custom_presets', JSON.stringify(custom));
  return [...mockPresets, ...custom];
}

export async function apiDeleteCustomPreset(presetId: string): Promise<Preset[]> {
  if (isTauri) {
    return await invoke<Preset[]>('delete_custom_preset', { presetId });
  }
  const local = localStorage.getItem('sonatune_custom_presets');
  let custom: Preset[] = local ? JSON.parse(local) : [];
  custom = custom.filter(p => p.id !== presetId);
  localStorage.setItem('sonatune_custom_presets', JSON.stringify(custom));
  return [...mockPresets, ...custom];
}

export async function apiGetAudioDevices(): Promise<AudioDeviceInfo[]> {
  if (isTauri) {
    return await invoke<AudioDeviceInfo[]>('get_audio_devices');
  }
  return [
    {
      id: 'Realtek High Definition Audio',
      name: 'Realtek High Definition Audio (Default Speakers)',
      is_default: true,
      max_channels: 2,
      default_sample_rate: 48000,
      supported_sample_rates: [44100, 48000, 96000],
      is_output: true,
    },
    {
      id: 'Headphones (High Definition Audio Device)',
      name: 'Headphones (High Definition Audio Device)',
      is_default: false,
      max_channels: 2,
      default_sample_rate: 48000,
      supported_sample_rates: [44100, 48000],
      is_output: true,
    },
  ];
}

export async function apiCheckMediaTools(): Promise<MediaToolInfo> {
  if (isTauri) {
    return await invoke<MediaToolInfo>('check_media_tools');
  }
  return {
    ffmpeg_available: false,
    ffprobe_available: false,
    version: undefined,
  };
}

export async function apiInspectAudio(filePath: string): Promise<AudioMetadata> {
  if (isTauri) {
    return await invoke<AudioMetadata>('inspect_audio', { filePath });
  }
  return {
    file_path: filePath,
    file_name: filePath.split(/[/\\]/).pop() || 'sample.wav',
    file_size_bytes: 14400000,
    format: 'WAV 16-bit PCM',
    duration_seconds: 180.5,
    sample_rate: 44100,
    channels: 2,
    peak_original: 0.85,
    rms_original: 0.22,
  };
}

export async function apiProcessAudio(
  inputPath: string,
  outputPath: string,
  config: DspConfig
): Promise<AudioProcessingResult> {
  if (isTauri) {
    return await invoke<AudioProcessingResult>('process_audio_file', {
      inputPath,
      outputPath,
      config,
    });
  }
  return {
    output_path: outputPath,
    duration_seconds: 180.5,
    peak_original: 0.85,
    rms_original: 0.22,
    peak_processed: 0.92,
    rms_processed: 0.26,
    loudness_gain_factor: 0.846,
  };
}

export async function apiCancelAudioProcessing(): Promise<void> {
  if (isTauri) {
    await invoke('cancel_audio_processing');
  }
}

export async function apiInspectVideo(filePath: string): Promise<VideoMetadata> {
  if (isTauri) {
    return await invoke<VideoMetadata>('inspect_video', { filePath });
  }
  return {
    file_path: filePath,
    file_name: filePath.split(/[/\\]/).pop() || 'sample.mp4',
    file_size_bytes: 45000000,
    duration_seconds: 124.0,
    video_streams: [{ index: 0, codec_name: 'h264', width: 1920, height: 1080, frame_rate: '30/1' }],
    audio_streams: [{ index: 1, codec_name: 'aac', channels: 2, sample_rate: 48000, language: 'eng', title: 'Main Stereo' }],
    subtitle_streams: [],
  };
}

export async function apiProcessVideo(
  sourceVideo: string,
  outputVideo: string,
  audioStreamIndex: number,
  config: DspConfig
): Promise<string> {
  if (isTauri) {
    return await invoke<string>('process_video_file', {
      sourceVideo,
      outputVideo,
      audioStreamIndex,
      config,
    });
  }
  return `Video successfully processed and remuxed to ${outputVideo}`;
}

export async function apiCancelVideoProcessing(): Promise<void> {
  if (isTauri) {
    await invoke('cancel_video_processing');
  }
}

export async function apiPlayerLoad(filePath: string): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_load_file', { path: filePath });
  }
  return {
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: true,
    file_path: filePath,
  };
}

export async function apiPlayerPlay(): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_play');
  }
  return {
    is_playing: true,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: true,
  };
}

export async function apiPlayerPause(): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_pause');
  }
  return {
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: true,
  };
}

export async function apiPlayerSeek(seconds: number): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_seek', { seconds });
  }
  return {
    is_playing: false,
    current_time_seconds: seconds,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: true,
  };
}

export async function apiPlayerGetState(): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_get_state');
  }
  return {
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: false,
  };
}

export async function apiPlayerSetAB(mode: 'original' | 'processed'): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_set_ab_mode', { mode });
  }
  return {
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: mode,
    loudness_matched: true,
    file_loaded: true,
  };
}

export async function apiPlayerSetLoudnessMatch(enabled: boolean, gainFactor: number): Promise<PlayerState> {
  if (isTauri) {
    return await invoke<PlayerState>('player_set_loudness_match', { enabled, gainFactor });
  }
  return {
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 180.5,
    a_b_mode: 'processed',
    loudness_matched: enabled,
    file_loaded: true,
  };
}

export async function apiGetLiveAudioStatus(): Promise<LiveAudioStatus> {
  if (isTauri) {
    return await invoke<LiveAudioStatus>('get_live_audio_status');
  }
  return {
    state: 'Stopped',
    sample_rate: 48000,
    channels: 2,
    estimated_latency_ms: 0,
    is_virtual_driver_present: false,
    feedback_protection_engaged: true,
    message: 'Standard physical audio endpoints active. To route all Windows app audio through SonaTune, a virtual audio device (such as VB-CABLE or Virtual Audio Cable) is recommended to prevent feedback.',
  };
}

export async function apiStartLiveAudio(inputDevice?: string, outputDevice?: string): Promise<LiveAudioStatus> {
  if (isTauri) {
    return await invoke<LiveAudioStatus>('start_live_audio', { inputDevice, outputDevice });
  }
  return {
    state: 'Running',
    input_device: inputDevice || 'Virtual Audio Cable (Loopback)',
    output_device: outputDevice || 'Speakers (Realtek)',
    sample_rate: 48000,
    channels: 2,
    estimated_latency_ms: 12.5,
    is_virtual_driver_present: true,
    feedback_protection_engaged: true,
    message: 'Active: live Windows audio is routed through SonaTune DSP processing.',
  };
}

export async function apiStopLiveAudio(): Promise<LiveAudioStatus> {
  if (isTauri) {
    return await invoke<LiveAudioStatus>('stop_live_audio');
  }
  return {
    state: 'Stopped',
    sample_rate: 48000,
    channels: 2,
    estimated_latency_ms: 0,
    is_virtual_driver_present: false,
    feedback_protection_engaged: true,
    message: 'Live audio routing stopped.',
  };
}
