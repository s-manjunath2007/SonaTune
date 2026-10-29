use crate::dsp::{DspConfig, DspEngine};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaToolInfo {
    pub ffmpeg_available: bool,
    pub ffprobe_available: bool,
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub format: String,
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_rate_kbps: Option<u32>,
    pub peak_original: f32,
    pub rms_original: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub language: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStreamInfo {
    pub index: usize,
    pub codec_name: String,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub duration_seconds: f64,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProcessingResult {
    pub output_path: String,
    pub duration_seconds: f64,
    pub peak_original: f32,
    pub rms_original: f32,
    pub peak_processed: f32,
    pub rms_processed: f32,
    pub loudness_gain_factor: f32, // Ratio for loudness matching preview
}

/// Locate FFmpeg binary on Windows
pub fn detect_ffmpeg() -> MediaToolInfo {
    let mut info = MediaToolInfo {
        ffmpeg_available: false,
        ffprobe_available: false,
        ffmpeg_path: None,
        ffprobe_path: None,
        version: None,
    };

    let mut candidate_dirs = vec![
        "".to_string(), // System PATH
        r"C:\ffmpeg\bin".to_string(),
        r"C:\Program Files\ffmpeg\bin".to_string(),
        r"C:\Program Files (x86)\ffmpeg\bin".to_string(),
    ];
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        candidate_dirs.push(format!(r"{}\Microsoft\WinGet\Links", local_app_data));
        candidate_dirs.push(format!(r"{}\Microsoft\WindowsApps", local_app_data));

        // Scan WinGet Packages directory for any installed FFmpeg package
        let winget_pkg_dir = std::path::PathBuf::from(&local_app_data).join(r"Microsoft\WinGet\Packages");
        if winget_pkg_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(winget_pkg_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name.to_lowercase().contains("ffmpeg") {
                            if let Ok(sub_entries) = std::fs::read_dir(&p) {
                                for sub in sub_entries.flatten() {
                                    let sub_p = sub.path();
                                    if sub_p.is_dir() {
                                        let bin = sub_p.join("bin");
                                        if bin.exists() {
                                            candidate_dirs.push(bin.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for dir in &candidate_dirs {
        let ffmpeg_cmd = if dir.is_empty() {
            "ffmpeg".to_string()
        } else {
            format!(r"{}\ffmpeg.exe", dir)
        };

        if let Ok(output) = Command::new(&ffmpeg_cmd).arg("-version").output() {
            if output.status.success() {
                info.ffmpeg_available = true;
                info.ffmpeg_path = Some(ffmpeg_cmd);
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = stdout.lines().next() {
                    info.version = Some(first_line.to_string());
                }
                break;
            }
        }
    }

    for dir in &candidate_dirs {
        let ffprobe_cmd = if dir.is_empty() {
            "ffprobe".to_string()
        } else {
            format!(r"{}\ffprobe.exe", dir)
        };

        if let Ok(output) = Command::new(&ffprobe_cmd).arg("-version").output() {
            if output.status.success() {
                info.ffprobe_available = true;
                info.ffprobe_path = Some(ffprobe_cmd);
                break;
            }
        }
    }

    info
}

/// Inspect audio file metadata using pure Rust Symphonia reader
pub fn inspect_audio_file(file_path: &str) -> Result<AudioMetadata, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let file_size_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .map_err(|e| format!("Unsupported or corrupted audio format: {e}"))?;

    let mut format = probed.format;
    let (sample_rate, channels, duration_seconds) = {
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "No supported audio tracks found in file".to_string())?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let duration_seconds = if let Some(n_frames) = track.codec_params.n_frames {
            n_frames as f64 / sample_rate as f64
        } else {
            0.0
        };
        (sample_rate, channels, duration_seconds)
    };

    let format_name = format.metadata().current().map(|_| "Audio").unwrap_or("Audio").to_string();

    Ok(AudioMetadata {
        file_path: file_path.to_string(),
        file_name,
        file_size_bytes,
        format: format_name,
        duration_seconds,
        sample_rate,
        channels,
        bit_rate_kbps: None,
        peak_original: 0.0,
        rms_original: 0.0,
    })
}

/// Decode audio into interleaved f32 samples using Symphonia
pub fn decode_audio_samples(file_path: &str) -> Result<(Vec<f32>, u32, u16), String> {
    let path = Path::new(file_path);
    let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| format!("Failed to probe audio format: {e}"))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| "No audio track found in file".to_string())?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| format!("Codec decoder creation failed: {e}"))?;

    let mut samples: Vec<f32> = Vec::new();

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        let planes = buf.planes();
                        let n_frames = buf.frames();
                        for frame_idx in 0..n_frames {
                            for plane in planes.planes() {
                                samples.push(plane[frame_idx]);
                            }
                        }
                    }
                    AudioBufferRef::S16(buf) => {
                        let planes = buf.planes();
                        let n_frames = buf.frames();
                        for frame_idx in 0..n_frames {
                            for plane in planes.planes() {
                                samples.push(plane[frame_idx] as f32 / 32768.0);
                            }
                        }
                    }
                    AudioBufferRef::S24(buf) => {
                        let planes = buf.planes();
                        let n_frames = buf.frames();
                        for frame_idx in 0..n_frames {
                            for plane in planes.planes() {
                                samples.push(plane[frame_idx].0 as f32 / 8388608.0);
                            }
                        }
                    }
                    AudioBufferRef::S32(buf) => {
                        let planes = buf.planes();
                        let n_frames = buf.frames();
                        for frame_idx in 0..n_frames {
                            for plane in planes.planes() {
                                samples.push(plane[frame_idx] as f32 / 2147483648.0);
                            }
                        }
                    }
                    AudioBufferRef::U8(buf) => {
                        let planes = buf.planes();
                        let n_frames = buf.frames();
                        for frame_idx in 0..n_frames {
                            for plane in planes.planes() {
                                samples.push((plane[frame_idx] as f32 - 128.0) / 128.0);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Decoding error: {e}")),
        }
    }

    Ok((samples, sample_rate, channels))
}

/// Calculate RMS and Peak level of audio buffer
pub fn calculate_levels(samples: &[f32]) -> (f32, f32) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sum_sq = 0.0f64;
    let mut peak = 0.0f32;

    for &s in samples {
        let abs = s.abs();
        if abs > peak {
            peak = abs;
        }
        sum_sq += (s as f64) * (s as f64);
    }

    let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
    (peak, rms)
}

/// Process an audio file with SonaTune DSP and export to WAV
pub fn process_and_export_audio(
    input_path: &str,
    output_path: &str,
    config: &DspConfig,
    cancel_flag: Arc<AtomicBool>,
) -> Result<AudioProcessingResult, String> {
    if input_path == output_path {
        return Err("Output file path must be different from source file path to prevent data loss.".to_string());
    }

    let (mut samples, sample_rate, channels) = decode_audio_samples(input_path)?;
    if samples.is_empty() {
        return Err("Decoded audio contains no audio frames.".to_string());
    }

    let (peak_orig, rms_orig) = calculate_levels(&samples);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Operation cancelled by user.".to_string());
    }

    let mut engine = DspEngine::new(sample_rate as f32);
    engine.set_config(config.clone());
    engine.process_interleaved(&mut samples, channels as usize);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Operation cancelled by user.".to_string());
    }

    let (peak_proc, rms_proc) = calculate_levels(&samples);

    // Loudness gain factor for accurate A/B comparison without volume bias
    let loudness_gain_factor = if rms_proc > 0.0001 && rms_orig > 0.0001 {
        rms_orig / rms_proc
    } else {
        1.0
    };

    // Determine output format based on extension
    let lower_out = output_path.to_lowercase();
    let is_wav = lower_out.ends_with(".wav");

    let temp_wav_path = if is_wav {
        output_path.to_string()
    } else {
        let temp_dir = std::env::temp_dir();
        temp_dir.join(format!("sonatune_temp_export_{}.wav", std::process::id())).to_string_lossy().to_string()
    };

    // Save intermediate or final output WAV using hound
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&temp_wav_path, spec)
        .map_err(|e| format!("Failed to create destination WAV file: {e}"))?;

    for s in &samples {
        let clamped = s.clamp(-1.0, 1.0);
        let sample_i16 = (clamped * 32767.0).round() as i16;
        writer
            .write_sample(sample_i16)
            .map_err(|e| format!("Error writing WAV samples: {e}"))?;
    }

    writer.finalize().map_err(|e| format!("Failed to finalize output WAV: {e}"))?;

    // If destination is not WAV (e.g. FLAC or MP3), convert via FFmpeg
    if !is_wav {
        let tools = detect_ffmpeg();
        let ffmpeg_cmd = match tools.ffmpeg_path {
            Some(cmd) => cmd,
            None => {
                let _ = std::fs::remove_file(&temp_wav_path);
                return Err(format!("Exporting to {} format requires FFmpeg. Please select WAV format for native lossless export, or install FFmpeg.",
                    Path::new(output_path).extension().and_then(|e| e.to_str()).unwrap_or("compressed").to_uppercase()));
            }
        };

        let mut ffmpeg_args = vec!["-y", "-i", &temp_wav_path];
        if lower_out.ends_with(".mp3") {
            ffmpeg_args.extend_from_slice(&["-c:a", "libmp3lame", "-b:a", "320k"]);
        } else if lower_out.ends_with(".flac") {
            ffmpeg_args.extend_from_slice(&["-c:a", "flac"]);
        } else if lower_out.ends_with(".ogg") {
            ffmpeg_args.extend_from_slice(&["-c:a", "libvorbis", "-q:a", "6"]);
        }
        ffmpeg_args.push(output_path);

        let conv_status = Command::new(&ffmpeg_cmd)
            .args(&ffmpeg_args)
            .status()
            .map_err(|e| format!("Failed to execute FFmpeg for format conversion: {e}"))?;

        let _ = std::fs::remove_file(&temp_wav_path);

        if !conv_status.success() {
            return Err("FFmpeg format encoding failed. Ensure target container is supported.".to_string());
        }
    }

    // Verify output file exists and has non-zero size
    let out_file = File::open(output_path).map_err(|e| format!("Failed to verify exported file: {e}"))?;
    let out_len = out_file.metadata().map(|m| m.len()).unwrap_or(0);
    if out_len < 100 {
        return Err("Exported file is empty or corrupted.".to_string());
    }

    let duration_seconds = (samples.len() / channels as usize) as f64 / sample_rate as f64;

    Ok(AudioProcessingResult {
        output_path: output_path.to_string(),
        duration_seconds,
        peak_original: peak_orig,
        rms_original: rms_orig,
        peak_processed: peak_proc,
        rms_processed: rms_proc,
        loudness_gain_factor,
    })
}

/// Inspect video streams using FFprobe
pub fn inspect_video_file(file_path: &str) -> Result<VideoMetadata, String> {
    let tools = detect_ffmpeg();
    let ffprobe_cmd = tools
        .ffprobe_path
        .ok_or_else(|| "FFprobe is not installed or not found in PATH. Please install FFmpeg to inspect video files.".to_string())?;

    let output = Command::new(ffprobe_cmd)
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            file_path,
        ])
        .output()
        .map_err(|e| format!("Failed to execute FFprobe: {e}"))?;

    if !output.status.success() {
        return Err("FFprobe failed to analyze video file. It may be corrupted or an unsupported container.".to_string());
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse FFprobe JSON: {e}"))?;

    let path = Path::new(file_path);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
    let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let duration_seconds = v["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    let mut subtitle_streams = Vec::new();

    if let Some(streams) = v["streams"].as_array() {
        for (idx, stream) in streams.iter().enumerate() {
            let codec_type = stream["codec_type"].as_str().unwrap_or("");
            let codec_name = stream["codec_name"].as_str().unwrap_or("unknown").to_string();

            match codec_type {
                "video" => {
                    let width = stream["width"].as_u64().unwrap_or(0) as u32;
                    let height = stream["height"].as_u64().unwrap_or(0) as u32;
                    let frame_rate = stream["r_frame_rate"].as_str().unwrap_or("").to_string();
                    video_streams.push(VideoStreamInfo {
                        index: idx,
                        codec_name,
                        width,
                        height,
                        frame_rate,
                    });
                }
                "audio" => {
                    let channels = stream["channels"].as_u64().unwrap_or(2) as u16;
                    let sample_rate = stream["sample_rate"]
                        .as_str()
                        .and_then(|s| s.parse::<u32>().ok())
                        .unwrap_or(48000);
                    let language = stream["tags"]["language"].as_str().map(|s| s.to_string());
                    let title = stream["tags"]["title"].as_str().map(|s| s.to_string());
                    audio_streams.push(AudioStreamInfo {
                        index: idx,
                        codec_name,
                        channels,
                        sample_rate,
                        language,
                        title,
                    });
                }
                "subtitle" => {
                    let language = stream["tags"]["language"].as_str().map(|s| s.to_string());
                    subtitle_streams.push(SubtitleStreamInfo {
                        index: idx,
                        codec_name,
                        language,
                    });
                }
                _ => {}
            }
        }
    }

    Ok(VideoMetadata {
        file_path: file_path.to_string(),
        file_name,
        file_size_bytes,
        duration_seconds,
        video_streams,
        audio_streams,
        subtitle_streams,
    })
}

/// Process video audio stream with DSP and remux with original video (no video re-encoding)
pub fn process_and_remux_video(
    source_video: &str,
    output_video: &str,
    audio_stream_index: usize,
    config: &DspConfig,
    cancel_flag: Arc<AtomicBool>,
) -> Result<String, String> {
    if source_video == output_video {
        return Err("Output video path must be different from source video path.".to_string());
    }

    let tools = detect_ffmpeg();
    let ffmpeg_cmd = tools
        .ffmpeg_path
        .ok_or_else(|| "FFmpeg is required for video audio remuxing but was not found.".to_string())?;

    // Temp file paths
    let temp_dir = std::env::temp_dir();
    let temp_extracted_wav = temp_dir.join(format!("sonatune_temp_in_{}.wav", std::process::id()));
    let temp_processed_wav = temp_dir.join(format!("sonatune_temp_out_{}.wav", std::process::id()));

    // Step 1: Extract selected audio stream using FFmpeg
    let extract_status = Command::new(&ffmpeg_cmd)
        .args([
            "-y",
            "-i", source_video,
            "-map", &format!("0:{}", audio_stream_index),
            "-vn",
            "-c:a", "pcm_s16le",
            temp_extracted_wav.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("Failed to extract audio track: {e}"))?;

    if !extract_status.success() {
        let _ = std::fs::remove_file(&temp_extracted_wav);
        return Err("FFmpeg failed to extract audio from video.".to_string());
    }

    if cancel_flag.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&temp_extracted_wav);
        return Err("Operation cancelled.".to_string());
    }

    // Step 2: Apply SonaTune DSP processing
    let proc_result = process_and_export_audio(
        temp_extracted_wav.to_str().unwrap(),
        temp_processed_wav.to_str().unwrap(),
        config,
        cancel_flag.clone(),
    );

    let _ = std::fs::remove_file(&temp_extracted_wav);

    if let Err(e) = proc_result {
        let _ = std::fs::remove_file(&temp_processed_wav);
        return Err(format!("Audio processing failed: {e}"));
    }

    if cancel_flag.load(Ordering::Relaxed) {
        let _ = std::fs::remove_file(&temp_processed_wav);
        return Err("Operation cancelled.".to_string());
    }

    // Step 3: Remux with original video stream (video is copied directly without recompression)
    // Preserve video stream (-c:v copy), enhanced audio, unselected audio tracks, subtitles (-c:s copy), and container metadata (-map_metadata 0).
    let mut remux_args = vec![
        "-y".to_string(),
        "-i".to_string(),
        source_video.to_string(),
        "-i".to_string(),
        temp_processed_wav.to_str().unwrap().to_string(),
        "-map".to_string(),
        "0:v?".to_string(),
        "-c:v".to_string(),
        "copy".to_string(),
    ];

    // Inspect audio tracks in source video to preserve any unselected tracks
    let mut mapped_enhanced = false;
    let mut out_audio_index = 0;
    if let Ok(meta) = inspect_video_file(source_video) {
        for a_stream in &meta.audio_streams {
            if a_stream.index == audio_stream_index {
                remux_args.push("-map".to_string());
                remux_args.push("1:a:0".to_string());
                remux_args.push(format!("-c:a:{out_audio_index}"));
                remux_args.push("aac".to_string());
                remux_args.push(format!("-b:a:{out_audio_index}"));
                remux_args.push("256k".to_string());
                mapped_enhanced = true;
            } else {
                remux_args.push("-map".to_string());
                remux_args.push(format!("0:{}", a_stream.index));
                remux_args.push(format!("-c:a:{out_audio_index}"));
                remux_args.push("copy".to_string());
            }
            out_audio_index += 1;
        }
    }

    if !mapped_enhanced {
        remux_args.push("-map".to_string());
        remux_args.push("1:a".to_string());
        remux_args.push("-c:a".to_string());
        remux_args.push("aac".to_string());
        remux_args.push("-b:a".to_string());
        remux_args.push("256k".to_string());
    }

    // Preserve subtitles, chapters, and container metadata
    remux_args.extend_from_slice(&[
        "-map".to_string(),
        "0:s?".to_string(),
        "-c:s".to_string(),
        "copy".to_string(),
        "-map_metadata".to_string(),
        "0".to_string(),
        "-map_chapters".to_string(),
        "0".to_string(),
        output_video.to_string(),
    ]);

    let remux_status = Command::new(&ffmpeg_cmd)
        .args(&remux_args)
        .status()
        .map_err(|e| format!("Failed to remux enhanced audio into video: {e}"))?;

    let _ = std::fs::remove_file(&temp_processed_wav);

    if !remux_status.success() {
        return Err("FFmpeg remuxing failed. Check output container compatibility.".to_string());
    }

    // Step 4: Verify output video file
    let out_meta = std::fs::metadata(output_video)
        .map_err(|e| format!("Failed to verify generated video: {e}"))?;
    if out_meta.len() < 1000 {
        return Err("Exported video file is invalid or empty.".to_string());
    }

    Ok(format!("Video successfully exported to {}", output_video))
}
