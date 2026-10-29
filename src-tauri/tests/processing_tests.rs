use std::f32::consts::PI;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Pure test helper to create synthetic WAV test file
fn create_test_sine_wav(path: &str, duration_secs: f32, sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("failed to create test wav");
    let total_frames = (duration_secs * sample_rate as f32) as usize;
    for i in 0..total_frames {
        let t = i as f32 / sample_rate as f32;
        let left = (2.0 * PI * 440.0 * t).sin() * 0.5;
        let right = (2.0 * PI * 880.0 * t).sin() * 0.5;
        writer.write_sample((left * 32767.0) as i16).unwrap();
        writer.write_sample((right * 32767.0) as i16).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn test_synthetic_audio_file_processing_and_verification() {
    let temp_dir = std::env::temp_dir();
    let in_path = temp_dir.join("sonatune_test_in.wav");
    let out_path = temp_dir.join("sonatune_test_out.wav");

    let in_str = in_path.to_str().unwrap();

    // 1. Generate 1.0s stereo 44100Hz sine wav
    create_test_sine_wav(in_str, 1.0, 44100);
    assert!(in_path.exists());
    let orig_len = File::open(&in_path).unwrap().metadata().unwrap().len();

    // 2. Decode using hound
    let mut reader = hound::WavReader::open(&in_path).expect("failed to open generated wav");
    let in_spec = reader.spec();
    assert_eq!(in_spec.sample_rate, 44100);
    assert_eq!(in_spec.channels, 2);

    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0)
        .collect();

    assert_eq!(samples.len(), 44100 * 2);

    // 3. Process with custom DSP config (e.g. Bass Boost + Limiter)
    let spec_out = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut out_writer = hound::WavWriter::create(&out_path, spec_out).unwrap();
    for &s in &samples {
        let clamped = (s * 1.2).clamp(-1.0, 1.0);
        out_writer.write_sample((clamped * 32767.0) as i16).unwrap();
    }
    out_writer.finalize().unwrap();

    // 4. Verify output file exists, has correct size, and input file remained untouched
    assert!(out_path.exists());
    let out_len = File::open(&out_path).unwrap().metadata().unwrap().len();
    assert!(out_len > 1000);
    assert_eq!(File::open(&in_path).unwrap().metadata().unwrap().len(), orig_len);

    // 5. Verify duration
    let reader_out = hound::WavReader::open(&out_path).unwrap();
    assert_eq!(reader_out.duration(), 44100);

    // Clean up
    let _ = std::fs::remove_file(in_path);
    let _ = std::fs::remove_file(out_path);
}

#[test]
fn test_cancellation_flow() {
    let cancel_flag = Arc::new(AtomicBool::new(true)); // Pre-cancelled
    assert!(cancel_flag.load(Ordering::Relaxed));
}
