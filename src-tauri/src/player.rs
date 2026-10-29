use crate::dsp::{DspConfig, DspEngine};
use crate::media::decode_audio_samples;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, Stream, StreamConfig};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub is_playing: bool,
    pub current_time_seconds: f64,
    pub total_duration_seconds: f64,
    pub a_b_mode: String, // "original" or "processed"
    pub loudness_matched: bool,
    pub file_loaded: bool,
    pub file_path: Option<String>,
}

enum PlayerCommand {
    Play { reply: Sender<Result<(), String>> },
    Pause { reply: Sender<()> },
    Stop { reply: Sender<()> },
    Shutdown,
}

pub struct AudioPlayer {
    cmd_tx: Sender<PlayerCommand>,
    samples: Arc<RwLock<Arc<Vec<f32>>>>,
    sample_rate: Arc<AtomicUsize>,
    channels: Arc<AtomicUsize>,
    cursor_frame: Arc<AtomicUsize>,
    is_playing: Arc<AtomicBool>,
    a_b_processed: Arc<AtomicBool>,
    loudness_match_enabled: Arc<AtomicBool>,
    loudness_gain: Arc<Mutex<f32>>,
    current_dsp_config: Arc<Mutex<DspConfig>>,
    current_file_path: Arc<Mutex<Option<String>>>,
}

// Sound COM & Real-Time Safety Invariants:
// 1. Thread Ownership: The CPAL WASAPI Stream is owned solely by 'sonatune-audio-player'.
//    No thread-affine COM interfaces ever cross thread boundaries.
// 2. Real-Time Safety: The audio render callback reads directly from an immutable Arc<Vec<f32>>
//    snapshot and writes in-place to data: &mut [f32]. No heap allocations (Vec::new), no blocking
//    file I/O, and no sample buffer lock contention occur in the audio callback.
// 3. 100% Safe Rust: No `unsafe` trait overrides are used.

impl AudioPlayer {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel::<PlayerCommand>();
        let samples = Arc::new(RwLock::new(Arc::new(Vec::new())));
        let sample_rate = Arc::new(AtomicUsize::new(44100));
        let channels = Arc::new(AtomicUsize::new(2));
        let cursor_frame = Arc::new(AtomicUsize::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let a_b_processed = Arc::new(AtomicBool::new(true));
        let loudness_match_enabled = Arc::new(AtomicBool::new(true));
        let loudness_gain = Arc::new(Mutex::new(1.0f32));
        let current_dsp_config = Arc::new(Mutex::new(DspConfig::default()));
        let current_file_path = Arc::new(Mutex::new(None));

        let t_samples = samples.clone();
        let t_sample_rate = sample_rate.clone();
        let t_channels = channels.clone();
        let t_cursor_frame = cursor_frame.clone();
        let t_is_playing = is_playing.clone();
        let t_a_b_processed = a_b_processed.clone();
        let t_loudness_match_enabled = loudness_match_enabled.clone();
        let t_loudness_gain = loudness_gain.clone();
        let t_current_dsp_config = current_dsp_config.clone();

        std::thread::Builder::new()
            .name("sonatune-audio-player".into())
            .spawn(move || {
                let mut current_stream: Option<Stream> = None;

                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        PlayerCommand::Play { reply } => {
                            if t_is_playing.load(Ordering::SeqCst) && current_stream.is_some() {
                                let _ = reply.send(Ok(()));
                                continue;
                            }

                            let active_track = t_samples.read().clone();
                            let total_samples = active_track.len();
                            let ch = t_channels.load(Ordering::SeqCst);
                            if total_samples == 0 || ch == 0 {
                                let _ = reply.send(Err("No audio loaded into player".to_string()));
                                continue;
                            }

                            let host = cpal::default_host();
                            let device = match host.default_output_device() {
                                Some(d) => d,
                                None => {
                                    let _ = reply.send(Err("No default audio output device found".to_string()));
                                    continue;
                                }
                            };

                            let sr = t_sample_rate.load(Ordering::SeqCst) as u32;
                            let stream_config = StreamConfig {
                                channels: 2.min(ch as u16),
                                sample_rate: SampleRate(sr),
                                buffer_size: BufferSize::Default,
                            };

                            let cursor_clone = t_cursor_frame.clone();
                            let is_playing_clone = t_is_playing.clone();
                            let a_b_proc_clone = t_a_b_processed.clone();
                            let l_match_clone = t_loudness_match_enabled.clone();
                            let l_gain_clone = t_loudness_gain.clone();
                            let dsp_config_clone = t_current_dsp_config.clone();

                            let mut dsp_engine = DspEngine::new(sr as f32);
                            let err_fn = |err| eprintln!("Player stream error: {err}");

                            // Real-time safe audio render callback (zero locks on sample buffer, zero heap allocations)
                            let stream_res = device.build_output_stream(
                                &stream_config,
                                move |data: &mut [f32], _: &_| {
                                    if !is_playing_clone.load(Ordering::Relaxed) {
                                        data.fill(0.0);
                                        return;
                                    }

                                    let total_len = active_track.len();
                                    let mut cursor = cursor_clone.load(Ordering::Relaxed);
                                    let frame_count = data.len() / 2;

                                    for i in 0..frame_count {
                                        if cursor * 2 + 1 < total_len {
                                            data[i * 2] = active_track[cursor * 2];
                                            data[i * 2 + 1] = active_track[cursor * 2 + 1];
                                            cursor += 1;
                                        } else {
                                            data[i * 2] = 0.0;
                                            data[i * 2 + 1] = 0.0;
                                            is_playing_clone.store(false, Ordering::Relaxed);
                                        }
                                    }

                                    cursor_clone.store(cursor, Ordering::Relaxed);

                                    if a_b_proc_clone.load(Ordering::Relaxed) {
                                        if let Some(cfg) = dsp_config_clone.try_lock() {
                                            dsp_engine.set_config(cfg.clone());
                                        }
                                        dsp_engine.process_interleaved(data, 2);

                                        if l_match_clone.load(Ordering::Relaxed) {
                                            if let Some(g) = l_gain_clone.try_lock() {
                                                let gain_val = *g;
                                                for s in data.iter_mut() {
                                                    *s *= gain_val;
                                                }
                                            }
                                        }
                                    }
                                },
                                err_fn,
                                None,
                            );

                            match stream_res {
                                Ok(stream) => {
                                    match stream.play() {
                                        Ok(_) => {
                                            current_stream = Some(stream);
                                            t_is_playing.store(true, Ordering::SeqCst);
                                            let _ = reply.send(Ok(()));
                                        }
                                        Err(e) => {
                                            let _ = reply.send(Err(format!("Failed to start playback stream: {e}")));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = reply.send(Err(format!("Failed to create audio output playback stream: {e}")));
                                }
                            }
                        }
                        PlayerCommand::Pause { reply } => {
                            t_is_playing.store(false, Ordering::SeqCst);
                            if let Some(stream) = current_stream.take() {
                                let _ = stream.pause();
                                drop(stream);
                            }
                            let _ = reply.send(());
                        }
                        PlayerCommand::Stop { reply } => {
                            t_is_playing.store(false, Ordering::SeqCst);
                            t_cursor_frame.store(0, Ordering::SeqCst);
                            if let Some(stream) = current_stream.take() {
                                let _ = stream.pause();
                                drop(stream);
                            }
                            let _ = reply.send(());
                        }
                        PlayerCommand::Shutdown => {
                            t_is_playing.store(false, Ordering::SeqCst);
                            if let Some(stream) = current_stream.take() {
                                let _ = stream.pause();
                                drop(stream);
                            }
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn audio player worker thread");

        Self {
            cmd_tx,
            samples,
            sample_rate,
            channels,
            cursor_frame,
            is_playing,
            a_b_processed,
            loudness_match_enabled,
            loudness_gain,
            current_dsp_config,
            current_file_path,
        }
    }

    pub fn load_file(&self, path: &str) -> Result<PlayerState, String> {
        self.stop();

        let (samples, sr, ch) = decode_audio_samples(path)?;
        if samples.is_empty() {
            return Err("Audio file contains no playable samples.".to_string());
        }

        *self.samples.write() = Arc::new(samples);
        self.sample_rate.store(sr as usize, Ordering::SeqCst);
        self.channels.store(ch as usize, Ordering::SeqCst);
        self.cursor_frame.store(0, Ordering::SeqCst);
        *self.current_file_path.lock() = Some(path.to_string());

        Ok(self.get_state())
    }

    pub fn update_dsp_config(&self, config: DspConfig) {
        *self.current_dsp_config.lock() = config;
    }

    pub fn set_a_b_mode(&self, mode: &str) {
        let is_proc = mode.to_lowercase() == "processed";
        self.a_b_processed.store(is_proc, Ordering::SeqCst);
    }

    pub fn set_loudness_matched(&self, enabled: bool) {
        self.loudness_match_enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn set_loudness_gain(&self, gain: f32) {
        *self.loudness_gain.lock() = gain;
    }

    pub fn play(&self) -> Result<PlayerState, String> {
        let (tx, rx) = channel();
        self.cmd_tx
            .send(PlayerCommand::Play { reply: tx })
            .map_err(|e| format!("Audio player worker thread unavailable: {e}"))?;
        rx.recv()
            .map_err(|e| format!("Worker thread response failed: {e}"))??;
        Ok(self.get_state())
    }

    pub fn pause(&self) -> PlayerState {
        let (tx, rx) = channel();
        if let Ok(_) = self.cmd_tx.send(PlayerCommand::Pause { reply: tx }) {
            let _ = rx.recv();
        }
        self.get_state()
    }

    pub fn stop(&self) {
        let (tx, rx) = channel();
        if let Ok(_) = self.cmd_tx.send(PlayerCommand::Stop { reply: tx }) {
            let _ = rx.recv();
        }
    }

    pub fn seek(&self, seconds: f64) -> PlayerState {
        let sr = self.sample_rate.load(Ordering::SeqCst) as f64;
        let frame = (seconds * sr).round().max(0.0) as usize;
        let max_frames = self.samples.read().len() / 2;
        self.cursor_frame.store(frame.min(max_frames), Ordering::SeqCst);
        self.get_state()
    }

    pub fn get_state(&self) -> PlayerState {
        let sr = self.sample_rate.load(Ordering::SeqCst) as f64;
        let total_samples = self.samples.read().len();
        let total_frames = if total_samples > 0 { total_samples / 2 } else { 0 };
        let current_frame = self.cursor_frame.load(Ordering::SeqCst);

        let total_dur = if sr > 0.0 { total_frames as f64 / sr } else { 0.0 };
        let cur_time = if sr > 0.0 { current_frame as f64 / sr } else { 0.0 };

        PlayerState {
            is_playing: self.is_playing.load(Ordering::SeqCst),
            current_time_seconds: cur_time.min(total_dur),
            total_duration_seconds: total_dur,
            a_b_mode: if self.a_b_processed.load(Ordering::SeqCst) {
                "processed".to_string()
            } else {
                "original".to_string()
            },
            loudness_matched: self.loudness_match_enabled.load(Ordering::SeqCst),
            file_loaded: total_samples > 0,
            file_path: self.current_file_path.lock().clone(),
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(PlayerCommand::Shutdown);
    }
}
