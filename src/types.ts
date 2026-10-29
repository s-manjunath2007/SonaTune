export interface DspConfig {
  enabled: boolean;
  master_volume: number;       // 0.0 to 2.0
  bass_gain_db: number;        // -12.0 to +12.0
  treble_gain_db: number;      // -12.0 to +12.0
  vocal_clarity_db: number;    // 0.0 to +12.0
  eq_bands_db: number[];       // 10 bands: 31Hz to 16kHz
  spatial_enabled: boolean;
  spatial_width: number;       // 0.0 to 1.0
  limiter_enabled: boolean;
  limiter_threshold_db: number;// -0.5 dB
}

export interface Preset {
  id: string;
  name: string;
  description: string;
  is_builtin: boolean;
  config: DspConfig;
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
  max_channels: number;
  default_sample_rate: number;
  supported_sample_rates: number[];
  is_output: boolean;
}

export interface DeviceStatus {
  connected: boolean;
  active_device_name: string;
  sample_rate: number;
  channels: number;
  latency_ms: number;
  error?: string;
}

export interface AudioMetadata {
  file_path: string;
  file_name: string;
  file_size_bytes: number;
  format: string;
  duration_seconds: number;
  sample_rate: number;
  channels: number;
  bit_rate_kbps?: number;
  peak_original: number;
  rms_original: number;
}

export interface VideoStreamInfo {
  index: number;
  codec_name: string;
  width: number;
  height: number;
  frame_rate: string;
}

export interface AudioStreamInfo {
  index: number;
  codec_name: string;
  channels: number;
  sample_rate: number;
  language?: string;
  title?: string;
}

export interface SubtitleStreamInfo {
  index: number;
  codec_name: string;
  language?: string;
}

export interface VideoMetadata {
  file_path: string;
  file_name: string;
  file_size_bytes: number;
  duration_seconds: number;
  video_streams: VideoStreamInfo[];
  audio_streams: AudioStreamInfo[];
  subtitle_streams: SubtitleStreamInfo[];
}

export interface AudioProcessingResult {
  output_path: string;
  duration_seconds: number;
  peak_original: number;
  rms_original: number;
  peak_processed: number;
  rms_processed: number;
  loudness_gain_factor: number;
}

export interface PlayerState {
  is_playing: boolean;
  current_time_seconds: number;
  total_duration_seconds: number;
  a_b_mode: 'original' | 'processed';
  loudness_matched: boolean;
  file_loaded: boolean;
  file_path?: string;
}

export interface LiveAudioStatus {
  state: 'Stopped' | 'Running' | 'Error';
  input_device?: string;
  output_device?: string;
  sample_rate: number;
  channels: number;
  estimated_latency_ms: number;
  is_virtual_driver_present: boolean;
  feedback_protection_engaged: boolean;
  message: string;
}

export interface MediaToolInfo {
  ffmpeg_available: boolean;
  ffprobe_available: boolean;
  ffmpeg_path?: string;
  ffprobe_path?: string;
  version?: string;
}

export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  last_page: string;
  active_preset_id: string;
  current_config: DspConfig;
  custom_presets: Preset[];
  selected_device_id?: string;
  device_profiles: Record<string, DspConfig>;
  last_export_dir?: string;
  minimize_to_tray: boolean;
  start_with_windows: boolean;
  a_b_loudness_matched: boolean;
}
