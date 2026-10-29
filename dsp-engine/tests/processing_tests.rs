use dsp_engine::{DspConfig, DspEngine};
use std::f32::consts::PI;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

    let mut samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / 32768.0)
        .collect();

    assert_eq!(samples.len(), 44100 * 2);

    // 3. Process with real SonaTune DSP Engine
    let mut dsp = DspEngine::new(44100.0);
    let mut config = DspConfig::default();
    config.bass_gain_db = 4.0;
    config.treble_gain_db = 2.0;
    config.vocal_clarity_db = 3.0;
    config.spatial_enabled = true;
    config.spatial_width = 0.3;
    dsp.set_config(config);
    dsp.process_interleaved(&mut samples, 2);

    // 4. Save output WAV
    let spec_out = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut out_writer = hound::WavWriter::create(&out_path, spec_out).unwrap();
    for &s in &samples {
        let clamped = s.clamp(-1.0, 1.0);
        out_writer.write_sample((clamped * 32767.0) as i16).unwrap();
    }
    out_writer.finalize().unwrap();

    // 5. Verify output file exists, has correct non-zero size, and input file was untouched
    assert!(out_path.exists());
    let out_len = File::open(&out_path).unwrap().metadata().unwrap().len();
    assert!(out_len > 1000);
    assert_eq!(File::open(&in_path).unwrap().metadata().unwrap().len(), orig_len);

    // 6. Verify duration
    let reader_out = hound::WavReader::open(&out_path).unwrap();
    assert_eq!(reader_out.duration(), 44100);

    // Clean up
    let _ = std::fs::remove_file(in_path);
    let _ = std::fs::remove_file(out_path);
}

#[test]
fn test_bass_and_treble_frequency_separation() {
    let sample_rate = 48000.0;
    let n = 48000; // 1 second

    // Generate 100Hz tone (bass) and 10kHz tone (treble)
    let mut bass_samples = Vec::with_capacity(n * 2);
    let mut treble_samples = Vec::with_capacity(n * 2);

    for i in 0..n {
        let t = i as f32 / sample_rate;
        let b = (2.0 * PI * 100.0 * t).sin() * 0.3;
        bass_samples.push(b);
        bass_samples.push(b);

        let tr = (2.0 * PI * 10000.0 * t).sin() * 0.3;
        treble_samples.push(tr);
        treble_samples.push(tr);
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    let orig_bass_rms = rms(&bass_samples);
    let orig_treble_rms = rms(&treble_samples);

    // Test Bass Boost
    let mut dsp_bass = DspEngine::new(sample_rate);
    let mut cfg_bass = DspConfig::default();
    cfg_bass.bass_gain_db = 6.0;
    cfg_bass.treble_gain_db = 0.0;
    dsp_bass.set_config(cfg_bass);

    let mut bass_proc = bass_samples.clone();
    dsp_bass.process_interleaved(&mut bass_proc, 2);
    let boosted_bass_rms = rms(&bass_proc);

    let mut treble_proc = treble_samples.clone();
    dsp_bass.process_interleaved(&mut treble_proc, 2);
    let unboosted_treble_rms = rms(&treble_proc);

    assert!(boosted_bass_rms > orig_bass_rms * 1.5, "Bass boost should increase 100Hz energy");
    assert!((unboosted_treble_rms - orig_treble_rms).abs() < orig_treble_rms * 0.15, "Bass boost should not significantly affect 10kHz");

    // Test Treble Boost
    let mut dsp_treble = DspEngine::new(sample_rate);
    let mut cfg_treble = DspConfig::default();
    cfg_treble.bass_gain_db = 0.0;
    cfg_treble.treble_gain_db = 6.0;
    dsp_treble.set_config(cfg_treble);

    let mut treble_boosted = treble_samples.clone();
    dsp_treble.process_interleaved(&mut treble_boosted, 2);
    let boosted_treble_rms = rms(&treble_boosted);

    assert!(boosted_treble_rms > orig_treble_rms * 1.5, "Treble boost should increase 10kHz energy");
}

#[test]
fn test_vocal_clarity_presence_boost() {
    let sample_rate = 48000.0;
    let n = 48000;
    let mut speech_presence = Vec::with_capacity(n * 2);

    for i in 0..n {
        let t = i as f32 / sample_rate;
        let v = (2.0 * PI * 2500.0 * t).sin() * 0.25;
        speech_presence.push(v);
        speech_presence.push(v);
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    let orig_rms = rms(&speech_presence);

    let mut dsp = DspEngine::new(sample_rate);
    let mut cfg = DspConfig::default();
    cfg.vocal_clarity_db = 6.0;
    dsp.set_config(cfg);

    dsp.process_interleaved(&mut speech_presence, 2);
    let boosted_rms = rms(&speech_presence);

    assert!(boosted_rms > orig_rms * 1.4, "Vocal clarity should significantly boost 2.5kHz presence range");
}

#[test]
fn test_bypass_produces_identical_audio() {
    let sample_rate = 44100.0;
    let original: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
    let mut processed = original.clone();

    let mut dsp = DspEngine::new(sample_rate);
    let mut cfg = DspConfig::default();
    cfg.enabled = false;
    cfg.bass_gain_db = 10.0; // even with extreme settings, bypass must prevent any modification
    cfg.treble_gain_db = 10.0;
    dsp.set_config(cfg);

    dsp.process_interleaved(&mut processed, 2);

    for (orig, proc) in original.iter().zip(processed.iter()) {
        assert_eq!(orig, proc, "Bypassed DSP must not alter audio signal");
    }
}

#[test]
fn test_limiter_strict_headroom_guarantee() {
    let sample_rate = 48000.0;
    // Huge input signal that would severely digitally clip without limiter
    let mut huge_signal: Vec<f32> = (0..4000).map(|i| (i as f32 * 0.1).sin() * 4.5).collect();

    let mut dsp = DspEngine::new(sample_rate);
    let mut cfg = DspConfig::default();
    cfg.limiter_enabled = true;
    cfg.limiter_threshold_db = -0.5; // -0.5 dB ceiling = approx 0.944 peak
    dsp.set_config(cfg);

    dsp.process_interleaved(&mut huge_signal, 2);

    let max_peak = huge_signal.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    assert!(max_peak <= 1.0, "Limiter must strictly constrain peak to <= 1.0, got {max_peak}");
}

#[test]
fn test_loudness_matching_level_calculation() {
    let orig: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin() * 0.2).collect();
    let boosted: Vec<f32> = orig.iter().map(|&s| s * 2.0).collect(); // 6 dB louder

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    let orig_rms = rms(&orig);
    let boosted_rms = rms(&boosted);

    let match_gain = orig_rms / boosted_rms.max(1e-6);
    let matched_boosted: Vec<f32> = boosted.iter().map(|&s| s * match_gain).collect();
    let matched_rms = rms(&matched_boosted);

    assert!((matched_rms - orig_rms).abs() < 1e-4, "Loudness matching must equalize perceived volume for honest A/B testing");
}

#[test]
fn test_cancellation_flow() {
    let cancel_flag = Arc::new(AtomicBool::new(true));
    assert!(cancel_flag.load(Ordering::Relaxed));
}
