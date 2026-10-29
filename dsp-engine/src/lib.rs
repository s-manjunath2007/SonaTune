use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// 10 Standard EQ band frequencies in Hz
pub const EQ_FREQUENCIES: [f32; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// DSP Processing parameters that can be updated from UI or presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DspConfig {
    pub enabled: bool,
    pub master_volume: f32,       // 0.0 to 2.0 (1.0 = unity, 0 dB)
    pub bass_gain_db: f32,        // -12.0 to +12.0 dB
    pub treble_gain_db: f32,      // -12.0 to +12.0 dB
    pub vocal_clarity_db: f32,    // 0.0 to +12.0 dB
    pub eq_bands_db: [f32; 10],   // -12.0 to +12.0 dB for each of the 10 bands
    pub spatial_enabled: bool,
    pub spatial_width: f32,       // 0.0 to 1.0 (subtle crossfeed and widening)
    pub limiter_enabled: bool,
    pub limiter_threshold_db: f32,// e.g. -0.5 dB
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 1.0,
            bass_gain_db: 0.0,
            treble_gain_db: 0.0,
            vocal_clarity_db: 0.0,
            eq_bands_db: [0.0; 10],
            spatial_enabled: false,
            spatial_width: 0.3,
            limiter_enabled: true,
            limiter_threshold_db: -0.5,
        }
    }
}

/// Biquad filter direct form II transposed implementation
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let clean_in = if input.abs() < 1e-15 { 0.0 } else { input };
        let out = self.b0 * clean_in + self.z1;
        self.z1 = self.b1 * clean_in - self.a1 * out + self.z2;
        self.z2 = self.b2 * clean_in - self.a2 * out;

        // Flush internal filter states to zero if below denormal threshold
        if self.z1.abs() < 1e-15 { self.z1 = 0.0; }
        if self.z2.abs() < 1e-15 { self.z2 = 0.0; }

        if out.is_nan() || out.is_infinite() {
            self.reset();
            return 0.0;
        }
        out
    }

    /// Peaking EQ filter (Robert Bristow-Johnson Audio EQ Cookbook)
    pub fn set_peaking_eq(&mut self, sample_rate: f32, center_freq: f32, gain_db: f32, q: f32) {
        if gain_db.abs() < 0.05 {
            // Identity
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let clamped_freq = center_freq.clamp(20.0, sample_rate * 0.48);
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * clamped_freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q);

        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * w0.cos()) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * w0.cos()) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    /// Low-shelf filter for bass boost/cut
    pub fn set_low_shelf(&mut self, sample_rate: f32, cutoff_freq: f32, gain_db: f32, s: f32) {
        if gain_db.abs() < 0.05 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let clamped_freq = cutoff_freq.clamp(20.0, sample_rate * 0.48);
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * clamped_freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).max(0.0).sqrt();

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha) / a0;
    }

    /// High-shelf filter for treble boost/cut
    pub fn set_high_shelf(&mut self, sample_rate: f32, cutoff_freq: f32, gain_db: f32, s: f32) {
        if gain_db.abs() < 0.05 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        let clamped_freq = cutoff_freq.clamp(20.0, sample_rate * 0.48);
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * clamped_freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 / s - 1.0) + 2.0).max(0.0).sqrt();

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        self.b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha)) / a0;
        self.b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        self.b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha)) / a0;
        self.a1 = (2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        self.a2 = ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha) / a0;
    }
}

/// Simple delay line for stereo spatial crossfeed / widening
#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
}

impl DelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples.max(1)],
            write_pos: 0,
            delay_samples: max_delay_samples / 2,
        }
    }

    pub fn set_delay(&mut self, samples: usize) {
        self.delay_samples = samples.clamp(0, self.buffer.len() - 1);
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let len = self.buffer.len();
        let read_pos = (self.write_pos + len - self.delay_samples) % len;
        let delayed = self.buffer[read_pos];
        self.buffer[self.write_pos] = input;
        self.write_pos = (self.write_pos + 1) % len;
        delayed
    }
}

/// Complete stereo/multi-channel DSP Engine
#[derive(Debug, Clone)]
pub struct DspEngine {
    sample_rate: f32,
    config: DspConfig,
    smoothed_master_volume: f32,

    eq_filters_l: [BiquadFilter; 10],
    eq_filters_r: [BiquadFilter; 10],
    eq_filters_c: [BiquadFilter; 10],

    bass_filter_l: BiquadFilter,
    bass_filter_r: BiquadFilter,
    bass_filter_c: BiquadFilter,

    treble_filter_l: BiquadFilter,
    treble_filter_r: BiquadFilter,
    treble_filter_c: BiquadFilter,

    vocal_filter_l: BiquadFilter,
    vocal_filter_r: BiquadFilter,
    vocal_filter_c: BiquadFilter,

    spatial_delay_l: DelayLine,
    spatial_delay_r: DelayLine,
}

impl DspEngine {
    pub fn new(sample_rate: f32) -> Self {
        let mut engine = Self {
            sample_rate,
            config: DspConfig::default(),
            smoothed_master_volume: 1.0,
            eq_filters_l: std::array::from_fn(|_| BiquadFilter::new()),
            eq_filters_r: std::array::from_fn(|_| BiquadFilter::new()),
            eq_filters_c: std::array::from_fn(|_| BiquadFilter::new()),
            bass_filter_l: BiquadFilter::new(),
            bass_filter_r: BiquadFilter::new(),
            bass_filter_c: BiquadFilter::new(),
            treble_filter_l: BiquadFilter::new(),
            treble_filter_r: BiquadFilter::new(),
            treble_filter_c: BiquadFilter::new(),
            vocal_filter_l: BiquadFilter::new(),
            vocal_filter_r: BiquadFilter::new(),
            vocal_filter_c: BiquadFilter::new(),
            spatial_delay_l: DelayLine::new((sample_rate * 0.005) as usize + 8),
            spatial_delay_r: DelayLine::new((sample_rate * 0.005) as usize + 8),
        };

        let delay_samples = (sample_rate * 0.00035).round() as usize;
        engine.spatial_delay_l.set_delay(delay_samples);
        engine.spatial_delay_r.set_delay(delay_samples);

        engine.recalculate_filters();
        engine
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            let delay_samples = (sample_rate * 0.00035).round() as usize;
            self.spatial_delay_l = DelayLine::new((sample_rate * 0.005) as usize + 8);
            self.spatial_delay_r = DelayLine::new((sample_rate * 0.005) as usize + 8);
            self.spatial_delay_l.set_delay(delay_samples);
            self.spatial_delay_r.set_delay(delay_samples);
            self.recalculate_filters();
        }
    }

    pub fn set_config(&mut self, config: DspConfig) {
        self.config = config;
        self.recalculate_filters();
    }

    pub fn get_config(&self) -> &DspConfig {
        &self.config
    }

    pub fn reset(&mut self) {
        for f in &mut self.eq_filters_l { f.reset(); }
        for f in &mut self.eq_filters_r { f.reset(); }
        for f in &mut self.eq_filters_c { f.reset(); }
        self.bass_filter_l.reset();
        self.bass_filter_r.reset();
        self.bass_filter_c.reset();
        self.treble_filter_l.reset();
        self.treble_filter_r.reset();
        self.treble_filter_c.reset();
        self.vocal_filter_l.reset();
        self.vocal_filter_r.reset();
        self.vocal_filter_c.reset();
        self.spatial_delay_l.reset();
        self.spatial_delay_r.reset();
        self.smoothed_master_volume = self.config.master_volume;
    }

    fn recalculate_filters(&mut self) {
        let sr = self.sample_rate;

        for i in 0..10 {
            let freq = EQ_FREQUENCIES[i];
            let gain = self.config.eq_bands_db[i];
            self.eq_filters_l[i].set_peaking_eq(sr, freq, gain, 1.414);
            self.eq_filters_r[i].set_peaking_eq(sr, freq, gain, 1.414);
            self.eq_filters_c[i].set_peaking_eq(sr, freq, gain, 1.414);
        }

        self.bass_filter_l.set_low_shelf(sr, 120.0, self.config.bass_gain_db, 1.0);
        self.bass_filter_r.set_low_shelf(sr, 120.0, self.config.bass_gain_db, 1.0);
        self.bass_filter_c.set_low_shelf(sr, 120.0, self.config.bass_gain_db, 1.0);

        self.treble_filter_l.set_high_shelf(sr, 7500.0, self.config.treble_gain_db, 1.0);
        self.treble_filter_r.set_high_shelf(sr, 7500.0, self.config.treble_gain_db, 1.0);
        self.treble_filter_c.set_high_shelf(sr, 7500.0, self.config.treble_gain_db, 1.0);

        self.vocal_filter_l.set_peaking_eq(sr, 3000.0, self.config.vocal_clarity_db, 1.3);
        self.vocal_filter_r.set_peaking_eq(sr, 3000.0, self.config.vocal_clarity_db, 1.3);
        self.vocal_filter_c.set_peaking_eq(sr, 3000.0, self.config.vocal_clarity_db, 1.3);
    }

    pub fn process_interleaved(&mut self, buffer: &mut [f32], channels: usize) {
        if !self.config.enabled {
            return;
        }

        let target_vol = self.config.master_volume;
        let smoothing_factor = 0.005;
        let limit_thresh = 10.0f32.powf(self.config.limiter_threshold_db / 20.0);

        if channels == 1 {
            for sample in buffer.iter_mut() {
                self.smoothed_master_volume += (target_vol - self.smoothed_master_volume) * smoothing_factor;
                let mut s = *sample;

                s = self.bass_filter_l.process(s);
                s = self.vocal_filter_l.process(s);
                s = self.treble_filter_l.process(s);

                for eq in &mut self.eq_filters_l {
                    s = eq.process(s);
                }

                s *= self.smoothed_master_volume;

                if self.config.limiter_enabled {
                    s = soft_limit(s, limit_thresh);
                }

                *sample = s;
            }
        } else if channels == 2 {
            let frame_count = buffer.len() / 2;
            let spatial_enabled = self.config.spatial_enabled;
            let spatial_w = self.config.spatial_width.clamp(0.0, 1.0);

            for i in 0..frame_count {
                self.smoothed_master_volume += (target_vol - self.smoothed_master_volume) * smoothing_factor;

                let mut l = buffer[i * 2];
                let mut r = buffer[i * 2 + 1];

                l = self.bass_filter_l.process(l);
                r = self.bass_filter_r.process(r);

                l = self.vocal_filter_l.process(l);
                r = self.vocal_filter_r.process(r);

                l = self.treble_filter_l.process(l);
                r = self.treble_filter_r.process(r);

                for band in 0..10 {
                    l = self.eq_filters_l[band].process(l);
                    r = self.eq_filters_r[band].process(r);
                }

                if spatial_enabled && spatial_w > 0.01 {
                    let delayed_l = self.spatial_delay_l.process(l);
                    let delayed_r = self.spatial_delay_r.process(r);

                    let cross_mix = spatial_w * 0.25;
                    let wide_mix = 1.0 + spatial_w * 0.25;

                    let new_l = (l * wide_mix - r * cross_mix + delayed_r * cross_mix * 0.5) / wide_mix;
                    let new_r = (r * wide_mix - l * cross_mix + delayed_l * cross_mix * 0.5) / wide_mix;

                    l = new_l;
                    r = new_r;
                }

                l *= self.smoothed_master_volume;
                r *= self.smoothed_master_volume;

                if self.config.limiter_enabled {
                    l = soft_limit(l, limit_thresh);
                    r = soft_limit(r, limit_thresh);
                }

                buffer[i * 2] = l;
                buffer[i * 2 + 1] = r;
            }
        } else if channels > 2 {
            let frame_count = buffer.len() / channels;
            let spatial_enabled = self.config.spatial_enabled;
            let spatial_w = self.config.spatial_width.clamp(0.0, 1.0);

            for i in 0..frame_count {
                self.smoothed_master_volume += (target_vol - self.smoothed_master_volume) * smoothing_factor;

                let offset = i * channels;
                let mut l = buffer[offset];
                let mut r = buffer[offset + 1];

                l = self.bass_filter_l.process(l);
                r = self.bass_filter_r.process(r);

                l = self.vocal_filter_l.process(l);
                r = self.vocal_filter_r.process(r);

                l = self.treble_filter_l.process(l);
                r = self.treble_filter_r.process(r);

                for band in 0..10 {
                    l = self.eq_filters_l[band].process(l);
                    r = self.eq_filters_r[band].process(r);
                }

                if spatial_enabled && spatial_w > 0.01 {
                    let delayed_l = self.spatial_delay_l.process(l);
                    let delayed_r = self.spatial_delay_r.process(r);

                    let cross_mix = spatial_w * 0.25;
                    let wide_mix = 1.0 + spatial_w * 0.25;

                    let new_l = (l * wide_mix - r * cross_mix + delayed_r * cross_mix * 0.5) / wide_mix;
                    let new_r = (r * wide_mix - l * cross_mix + delayed_l * cross_mix * 0.5) / wide_mix;

                    l = new_l;
                    r = new_r;
                }

                l *= self.smoothed_master_volume;
                r *= self.smoothed_master_volume;

                if self.config.limiter_enabled {
                    l = soft_limit(l, limit_thresh);
                    r = soft_limit(r, limit_thresh);
                }

                buffer[offset] = l;
                buffer[offset + 1] = r;

                // Process Channel 2 (Front Center - carries dialogue in 5.1/7.1 film and video)
                if channels >= 3 {
                    let mut c = buffer[offset + 2];
                    c = self.bass_filter_c.process(c);
                    c = self.vocal_filter_c.process(c);
                    c = self.treble_filter_c.process(c);
                    for eq in &mut self.eq_filters_c {
                        c = eq.process(c);
                    }
                    c *= self.smoothed_master_volume;
                    if self.config.limiter_enabled {
                        c = soft_limit(c, limit_thresh);
                    }
                    buffer[offset + 2] = c;
                }

                // Process remaining channels (Channel 3 is LFE/Subwoofer, Channel 4+ are Surround L/R, Back L/R)
                for ch in 3..channels {
                    let mut s = buffer[offset + ch] * self.smoothed_master_volume;
                    if self.config.limiter_enabled {
                        s = soft_limit(s, limit_thresh);
                    }
                    buffer[offset + ch] = s;
                }
            }
        }
    }
}

#[inline(always)]
fn soft_limit(x: f32, threshold: f32) -> f32 {
    if x.is_nan() || x.is_infinite() {
        return 0.0;
    }
    let safe_threshold = threshold.clamp(0.01, 0.999);
    let knee_start = safe_threshold * 0.85;
    let abs_x = x.abs();
    if abs_x <= knee_start {
        x
    } else {
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        let excess = abs_x - knee_start;
        let headroom = safe_threshold - knee_start;
        let limited = knee_start + headroom * (excess / headroom).tanh();
        sign * limited.min(safe_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain_changes_signal() {
        let mut dsp = DspEngine::new(48000.0);
        let mut config = DspConfig::default();
        config.master_volume = 1.5;
        config.limiter_enabled = false;
        dsp.set_config(config);

        let mut buf = vec![0.5, -0.5];
        dsp.process_interleaved(&mut buf, 2);

        assert!(buf[0] > 0.5);
        assert!(buf[1] < -0.5);
    }

    #[test]
    fn test_silence_handling() {
        let mut dsp = DspEngine::new(44100.0);
        let mut buf = vec![0.0; 512];
        dsp.process_interleaved(&mut buf, 2);
        for s in buf {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_very_small_signals_and_denormals() {
        let mut dsp = DspEngine::new(48000.0);
        let mut buf = vec![1e-15, -1e-15, 1e-20, -1e-20];
        dsp.process_interleaved(&mut buf, 2);
        for s in buf {
            assert!(!s.is_nan());
            assert!(!s.is_infinite());
        }
    }

    #[test]
    fn test_finite_output_and_stability() {
        let mut dsp = DspEngine::new(48000.0);
        let mut config = DspConfig::default();
        config.bass_gain_db = 12.0;
        config.treble_gain_db = 12.0;
        config.vocal_clarity_db = 12.0;
        config.eq_bands_db = [12.0; 10];
        dsp.set_config(config);

        let mut buf = vec![0.0; 2048];
        buf[0] = 1.0;
        buf[1] = 1.0;
        dsp.process_interleaved(&mut buf, 2);

        for s in buf {
            assert!(!s.is_nan());
            assert!(!s.is_infinite());
            assert!(s.abs() <= 1.0);
        }
    }

    #[test]
    fn test_large_signal_limiter() {
        let mut dsp = DspEngine::new(48000.0);
        let mut config = DspConfig::default();
        config.limiter_enabled = true;
        config.master_volume = 2.0;
        dsp.set_config(config);

        let mut buf = vec![1.5, -2.0, 5.0, -10.0];
        dsp.process_interleaved(&mut buf, 2);

        for s in buf {
            assert!(s.abs() <= 1.0, "Limiter must clamp value: {}", s);
        }
    }

    #[test]
    fn test_stereo_and_spatial() {
        let mut dsp = DspEngine::new(48000.0);
        let mut config = DspConfig::default();
        config.spatial_enabled = true;
        config.spatial_width = 0.5;
        dsp.set_config(config);

        let mut buf = vec![0.5, 0.0, 0.5, 0.0, 0.5, 0.0];
        dsp.process_interleaved(&mut buf, 2);

        assert!(!buf[0].is_nan());
        assert!(!buf[1].is_nan());
    }

    #[test]
    fn test_multiple_sample_rates() {
        for sr in [22050.0, 44100.0, 48000.0, 88200.0, 96000.0, 192000.0] {
            let mut dsp = DspEngine::new(sr);
            let mut buf = vec![0.1; 256];
            dsp.process_interleaved(&mut buf, 2);
            assert!(!buf[0].is_nan());
        }
    }

    #[test]
    fn test_multichannel_5_1_surround() {
        let mut dsp = DspEngine::new(48000.0);
        let mut config = DspConfig::default();
        config.master_volume = 1.0;
        config.vocal_clarity_db = 6.0;
        config.limiter_enabled = false;
        dsp.set_config(config);

        // 6 channels: FL, FR, C, LFE, SL, SR (64 frames = 384 samples)
        let channels = 6;
        let mut buf = vec![0.0; 256 * channels];
        // Feed 3 kHz sine wave into Center channel (channel index 2)
        for i in 0..256 {
            let t = i as f32 / 48000.0;
            buf[i * channels + 2] = (2.0 * std::f32::consts::PI * 3000.0 * t).sin() * 0.1;
        }

        dsp.process_interleaved(&mut buf, channels);

        // Center channel amplitude should be boosted by the +6dB vocal clarity filter
        let mut max_center = 0.0f32;
        for i in 30..256 {
            let val = buf[i * channels + 2].abs();
            if val > max_center { max_center = val; }
        }
        assert!(max_center > 0.15, "5.1 Center channel must be boosted by vocal clarity filter, got peak {}", max_center);
    }
}
