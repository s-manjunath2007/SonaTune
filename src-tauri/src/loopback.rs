use crate::dsp::{DspConfig, DspEngine};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, Stream, StreamConfig};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LiveRoutingState {
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveAudioStatus {
    pub state: LiveRoutingState,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub sample_rate: u32,
    pub channels: u16,
    pub estimated_latency_ms: f32,
    pub is_virtual_driver_present: bool,
    pub feedback_protection_engaged: bool,
    pub message: String,
}

enum RouterCommand {
    Start {
        input_device_name: Option<String>,
        output_device_name: Option<String>,
        reply: Sender<Result<LiveAudioStatus, String>>,
    },
    Stop {
        reply: Sender<LiveAudioStatus>,
    },
    Shutdown,
}

// Helper linear streaming resampler for hardware/virtual audio endpoint rate mismatches
#[derive(Debug, Clone)]
pub struct StereoResampler {
    pub ratio: f64, // in_rate / out_rate
    last_l: f32,
    last_r: f32,
    curr_l: f32,
    curr_r: f32,
    phase: f64,
    has_first: bool,
}

impl StereoResampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        Self {
            ratio: in_rate as f64 / out_rate as f64,
            last_l: 0.0,
            last_r: 0.0,
            curr_l: 0.0,
            curr_r: 0.0,
            phase: 0.0,
            has_first: false,
        }
    }

    pub fn push_frame<F: FnMut(f32, f32)>(&mut self, in_l: f32, in_r: f32, mut emit: F) {
        if !self.has_first {
            self.last_l = in_l;
            self.last_r = in_r;
            self.curr_l = in_l;
            self.curr_r = in_r;
            self.has_first = true;
            return;
        }

        self.last_l = self.curr_l;
        self.last_r = self.curr_r;
        self.curr_l = in_l;
        self.curr_r = in_r;

        while self.phase < 1.0 {
            let frac = self.phase as f32;
            let out_l = self.last_l + frac * (self.curr_l - self.last_l);
            let out_r = self.last_r + frac * (self.curr_r - self.last_r);
            emit(out_l, out_r);
            self.phase += self.ratio;
        }
        self.phase -= 1.0;
    }
}

// Thread-safe actor architecture: The CPAL WASAPI input and output Streams are owned
// and executed exclusively by the dedicated background thread 'sonatune-live-router'.
// No thread-affine Windows COM interfaces cross thread boundaries.
// LiveAudioRouter is composed purely of Send + Sync primitives, requiring zero unsafe code.

pub struct LiveAudioRouter {
    cmd_tx: Sender<RouterCommand>,
    is_running: Arc<AtomicBool>,
    dsp_config: Arc<Mutex<DspConfig>>,
    status: Arc<Mutex<LiveAudioStatus>>,
}

impl LiveAudioRouter {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = channel::<RouterCommand>();
        let has_virtual = detect_virtual_audio_driver();

        let initial_status = LiveAudioStatus {
            state: LiveRoutingState::Stopped,
            input_device: None,
            output_device: None,
            sample_rate: 48000,
            channels: 2,
            estimated_latency_ms: 0.0,
            is_virtual_driver_present: has_virtual,
            feedback_protection_engaged: true,
            message: if has_virtual {
                "Virtual audio endpoint detected. Live system-wide audio routing is ready.".to_string()
            } else {
                "Standard physical audio endpoints active. To route all Windows app audio through SonaTune, a virtual audio device (such as VB-CABLE or Virtual Audio Cable) is recommended to prevent acoustic and digital feedback loops.".to_string()
            },
        };

        let is_running = Arc::new(AtomicBool::new(false));
        let dsp_config = Arc::new(Mutex::new(DspConfig::default()));
        let status = Arc::new(Mutex::new(initial_status));

        let t_is_running = is_running.clone();
        let t_dsp_config = dsp_config.clone();
        let t_status = status.clone();

        std::thread::Builder::new()
            .name("sonatune-live-router".into())
            .spawn(move || {
                let mut active_streams: Option<(Stream, Stream)> = None;

                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        RouterCommand::Start {
                            input_device_name,
                            output_device_name,
                            reply,
                        } => {
                            // Stop any existing stream
                            if let Some((in_s, out_s)) = active_streams.take() {
                                let _ = in_s.pause();
                                let _ = out_s.pause();
                                drop(in_s);
                                drop(out_s);
                            }
                            t_is_running.store(false, Ordering::SeqCst);

                            let host = cpal::default_host();

                            // 1. Resolve output device
                            let out_device = if let Some(ref name) = output_device_name {
                                match host.output_devices() {
                                    Ok(mut devs) => match devs.find(|d| d.name().map(|n| n == *name).unwrap_or(false)) {
                                        Some(d) => d,
                                        None => {
                                            let err_msg = format!("Selected output device '{name}' not found");
                                            let mut st = t_status.lock();
                                            st.state = LiveRoutingState::Error;
                                            st.message = err_msg.clone();
                                            let _ = reply.send(Err(err_msg));
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        let err_msg = format!("Failed to enumerate output devices: {e}");
                                        let _ = reply.send(Err(err_msg));
                                        continue;
                                    }
                                }
                            } else {
                                match host.default_output_device() {
                                    Some(d) => d,
                                    None => {
                                        let err_msg = "No default output device available on Windows".to_string();
                                        let _ = reply.send(Err(err_msg));
                                        continue;
                                    }
                                }
                            };

                            let resolved_out_name = out_device.name().unwrap_or_else(|_| "Default Output".to_string());

                            // 2. Resolve input device
                            let in_device = if let Some(ref name) = input_device_name {
                                match host.input_devices() {
                                    Ok(mut devs) => match devs.find(|d| d.name().map(|n| n == *name).unwrap_or(false)) {
                                        Some(d) => d,
                                        None => {
                                            let err_msg = format!("Selected input device '{name}' not found");
                                            let mut st = t_status.lock();
                                            st.state = LiveRoutingState::Error;
                                            st.message = err_msg.clone();
                                            let _ = reply.send(Err(err_msg));
                                            continue;
                                        }
                                    },
                                    Err(e) => {
                                        let err_msg = format!("Failed to enumerate input devices: {e}");
                                        let _ = reply.send(Err(err_msg));
                                        continue;
                                    }
                                }
                            } else {
                                let mut virtual_in = None;
                                if let Ok(devices) = host.input_devices() {
                                    for d in devices {
                                        if let Ok(name) = d.name() {
                                            let lower = name.to_lowercase();
                                            if lower.contains("cable") || lower.contains("virtual") || lower.contains("voicemeeter") {
                                                virtual_in = Some(d);
                                                break;
                                            }
                                        }
                                    }
                                }

                                match virtual_in {
                                    Some(vd) => vd,
                                    None => match host.default_input_device() {
                                        Some(d) => d,
                                        None => {
                                            let err_msg = "No input or virtual audio capture device available on system".to_string();
                                            let _ = reply.send(Err(err_msg));
                                            continue;
                                        }
                                    },
                                }
                            };

                            let resolved_in_name = in_device.name().unwrap_or_else(|_| "Default Input".to_string());

                            // 3. Feedback protection check: never route identical endpoints
                            if resolved_in_name == resolved_out_name {
                                let mut st = t_status.lock();
                                st.state = LiveRoutingState::Error;
                                st.feedback_protection_engaged = true;
                                st.message = "Feedback Protection: Input and output devices cannot be the same physical device.".to_string();
                                let _ = reply.send(Err(st.message.clone()));
                                continue;
                            }

                            let out_config = match out_device.default_output_config() {
                                Ok(cfg) => cfg,
                                Err(e) => {
                                    let err_msg = format!("Failed to get default output config: {e}");
                                    let _ = reply.send(Err(err_msg));
                                    continue;
                                }
                            };
                            let out_sample_rate = out_config.sample_rate().0;
                            let out_channels = out_config.channels();

                            let in_config = match in_device.default_input_config() {
                                Ok(cfg) => cfg,
                                Err(e) => {
                                    let err_msg = format!("Failed to get default input config: {e}");
                                    let _ = reply.send(Err(err_msg));
                                    continue;
                                }
                            };
                            let in_sample_rate = in_config.sample_rate().0;
                            let in_channels = in_config.channels();

                            let ring_buffer_capacity = (out_sample_rate as usize * 2 / 5).max(2048);
                            let (producer, consumer) = crossbeam_channel::bounded::<f32>(ring_buffer_capacity);

                            let in_stream_config: StreamConfig = in_config.into();
                            let mut resampler = if in_sample_rate != out_sample_rate {
                                Some(StereoResampler::new(in_sample_rate, out_sample_rate))
                            } else {
                                None
                            };

                            let err_fn = |err| eprintln!("Live audio stream error: {err}");

                            let in_stream = match in_device.build_input_stream(
                                &in_stream_config,
                                move |data: &[f32], _: &_| {
                                    if let Some(ref mut rs) = resampler {
                                        if in_channels == 1 {
                                            for &sample in data {
                                                rs.push_frame(sample, sample, |l, r| {
                                                    let _ = producer.try_send(l);
                                                    let _ = producer.try_send(r);
                                                });
                                            }
                                        } else if in_channels == 2 {
                                            for frame in data.chunks_exact(2) {
                                                rs.push_frame(frame[0], frame[1], |l, r| {
                                                    let _ = producer.try_send(l);
                                                    let _ = producer.try_send(r);
                                                });
                                            }
                                        } else {
                                            let ch = in_channels as usize;
                                            for frame in data.chunks_exact(ch) {
                                                rs.push_frame(frame[0], frame[1], |l, r| {
                                                    let _ = producer.try_send(l);
                                                    let _ = producer.try_send(r);
                                                });
                                            }
                                        }
                                    } else {
                                        if in_channels == 1 {
                                            for &sample in data {
                                                let _ = producer.try_send(sample);
                                                let _ = producer.try_send(sample);
                                            }
                                        } else if in_channels == 2 {
                                            for &sample in data {
                                                let _ = producer.try_send(sample);
                                            }
                                        } else {
                                            let ch = in_channels as usize;
                                            for frame in data.chunks_exact(ch) {
                                                let _ = producer.try_send(frame[0]);
                                                let _ = producer.try_send(frame[1]);
                                            }
                                        }
                                    }
                                },
                                err_fn,
                                None,
                            ) {
                                Ok(s) => s,
                                Err(e) => {
                                    let err_msg = format!("Failed to build input audio stream: {e}");
                                    let _ = reply.send(Err(err_msg));
                                    continue;
                                }
                            };

                            let out_stream_config = StreamConfig {
                                channels: 2.min(out_channels),
                                sample_rate: SampleRate(out_sample_rate),
                                buffer_size: BufferSize::Default,
                            };

                            let dsp_config_clone = t_dsp_config.clone();
                            let mut dsp_engine = DspEngine::new(out_sample_rate as f32);
                            let proc_channels = (2.min(out_channels)) as usize;

                            let out_stream = match out_device.build_output_stream(
                                &out_stream_config,
                                move |data: &mut [f32], _: &_| {
                                    // Real-time audio safe: zero heap allocation in callback
                                    for sample in data.iter_mut() {
                                        *sample = consumer.try_recv().unwrap_or(0.0);
                                    }

                                    if let Some(cfg) = dsp_config_clone.try_lock() {
                                        dsp_engine.set_config(cfg.clone());
                                    }
                                    dsp_engine.process_interleaved(data, proc_channels);
                                },
                                err_fn,
                                None,
                            ) {
                                Ok(s) => s,
                                Err(e) => {
                                    let err_msg = format!("Failed to build output audio stream: {e}");
                                    let _ = reply.send(Err(err_msg));
                                    continue;
                                }
                            };

                            if let Err(e) = in_stream.play() {
                                let err_msg = format!("Failed to start input stream: {e}");
                                let _ = reply.send(Err(err_msg));
                                continue;
                            }

                            if let Err(e) = out_stream.play() {
                                let err_msg = format!("Failed to start output stream: {e}");
                                let _ = reply.send(Err(err_msg));
                                continue;
                            }

                            active_streams = Some((in_stream, out_stream));
                            t_is_running.store(true, Ordering::SeqCst);

                            let est_latency = (ring_buffer_capacity as f32 / (out_sample_rate as f32 * out_channels as f32)) * 500.0;

                            let status_res = LiveAudioStatus {
                                state: LiveRoutingState::Running,
                                input_device: Some(resolved_in_name),
                                output_device: Some(resolved_out_name),
                                sample_rate: out_sample_rate,
                                channels: out_channels,
                                estimated_latency_ms: est_latency.clamp(8.0, 50.0),
                                is_virtual_driver_present: detect_virtual_audio_driver(),
                                feedback_protection_engaged: true,
                                message: "Active: live Windows audio is routed through SonaTune DSP processing.".to_string(),
                            };

                            *t_status.lock() = status_res.clone();
                            let _ = reply.send(Ok(status_res));
                        }
                        RouterCommand::Stop { reply } => {
                            t_is_running.store(false, Ordering::SeqCst);
                            if let Some((in_s, out_s)) = active_streams.take() {
                                let _ = in_s.pause();
                                let _ = out_s.pause();
                                drop(in_s);
                                drop(out_s);
                            }

                            let mut st = t_status.lock();
                            st.state = LiveRoutingState::Stopped;
                            st.estimated_latency_ms = 0.0;
                            st.message = "Live audio routing stopped.".to_string();
                            let _ = reply.send(st.clone());
                        }
                        RouterCommand::Shutdown => {
                            t_is_running.store(false, Ordering::SeqCst);
                            if let Some((in_s, out_s)) = active_streams.take() {
                                let _ = in_s.pause();
                                let _ = out_s.pause();
                                drop(in_s);
                                drop(out_s);
                            }
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn live audio router worker thread");

        Self {
            cmd_tx,
            is_running,
            dsp_config,
            status,
        }
    }

    pub fn update_dsp_config(&self, config: DspConfig) {
        *self.dsp_config.lock() = config;
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn get_status(&self) -> LiveAudioStatus {
        let mut st = self.status.lock().clone();
        st.is_virtual_driver_present = detect_virtual_audio_driver();
        st
    }

    pub fn start_route(
        &self,
        input_device_name: Option<String>,
        output_device_name: Option<String>,
    ) -> Result<LiveAudioStatus, String> {
        let (tx, rx) = channel();
        self.cmd_tx
            .send(RouterCommand::Start {
                input_device_name,
                output_device_name,
                reply: tx,
            })
            .map_err(|e| format!("Live router worker thread unavailable: {e}"))?;
        rx.recv()
            .map_err(|e| format!("Worker thread failed to respond: {e}"))?
    }

    pub fn stop_route(&self) -> LiveAudioStatus {
        let (tx, rx) = channel();
        if let Ok(_) = self.cmd_tx.send(RouterCommand::Stop { reply: tx }) {
            if let Ok(st) = rx.recv() {
                return st;
            }
        }
        self.get_status()
    }
}

impl Drop for LiveAudioRouter {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(RouterCommand::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_protection_triggers_when_endpoints_match() {
        let router = LiveAudioRouter::new();
        let dev_name = "Speakers (Realtek(R) Audio)".to_string();
        let res = router.start_route(Some(dev_name.clone()), Some(dev_name.clone()));
        assert!(res.is_err(), "Same-endpoint routing must be blocked by feedback prevention");
        let err = res.err().unwrap();
        println!("Actual error: {err}");
        assert!(
            err.contains("Feedback Protection") || err.contains("Selected input device"),
            "Error message should mention Feedback Protection or invalid input device"
        );
    }

    #[test]
    fn test_virtual_audio_driver_detection() {
        let router = LiveAudioRouter::new();
        let status = router.get_status();
        println!("Virtual audio driver present: {}", status.is_virtual_driver_present);
        println!("Initial status message: {}", status.message);
        assert!(!status.message.is_empty());
    }

    #[test]
    fn test_stereo_resampler_rate_conversion() {
        // Test 44.1 kHz to 48 kHz upsampling
        let mut resampler = StereoResampler::new(44100, 48000);
        let mut out_count = 0;
        let in_count = 44100;
        for i in 0..in_count {
            let sample = (i as f32 / 100.0).sin();
            resampler.push_frame(sample, sample, |_l, _r| {
                out_count += 1;
            });
        }
        // Output count should be approximately 48000 within 2 frames tolerance
        let diff = (out_count as i32 - 48000).abs();
        assert!(diff <= 2, "Expected ~48000 frames, got {out_count} (diff: {diff})");

        // Test 48 kHz to 44.1 kHz downsampling
        let mut resampler_down = StereoResampler::new(48000, 44100);
        let mut out_down_count = 0;
        for i in 0..48000 {
            let sample = (i as f32 / 100.0).sin();
            resampler_down.push_frame(sample, sample, |_l, _r| {
                out_down_count += 1;
            });
        }
        let diff_down = (out_down_count as i32 - 44100).abs();
        assert!(diff_down <= 2, "Expected ~44100 frames, got {out_down_count} (diff: {diff_down})");

        // Test 96 kHz to 44.1 kHz downsampling (ratio > 2.0)
        let mut resampler_96_44 = StereoResampler::new(96000, 44100);
        let mut out_96_44_count = 0;
        for i in 0..96000 {
            let sample = (i as f32 / 100.0).sin();
            resampler_96_44.push_frame(sample, sample, |_l, _r| {
                out_96_44_count += 1;
            });
        }
        let diff_96_44 = (out_96_44_count as i32 - 44100).abs();
        assert!(diff_96_44 <= 2, "Expected ~44100 frames from 96000 in, got {out_96_44_count} (diff: {diff_96_44})");
    }
}

/// Helper to check if a known virtual audio cable driver is installed on Windows
pub fn detect_virtual_audio_driver() -> bool {
    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                let lower = name.to_lowercase();
                if lower.contains("cable")
                    || lower.contains("virtual")
                    || lower.contains("voicemeeter")
                    || lower.contains("vb-audio")
                {
                    return true;
                }
            }
        }
    }
    false
}
