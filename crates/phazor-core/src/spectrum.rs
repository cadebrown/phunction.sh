//! A full-resolution spectrum analyzer that lives on the audio thread.
//!
//! Ninety-six Goertzel resonators at log-spaced centers (50 Hz → 14 kHz),
//! evaluated per block and smoothed with fast-attack/slow-release — the
//! classic analyzer ballistics. Cost: 96 × `block_len` multiply-adds
//! (~50k ops/block), still far inside the realtime budget, and no FFT
//! scratch memory at all.

use crate::Sample;

/// Number of analyzer bands — an eighth-octave grid, dense enough that the
/// UI draws a curve instead of a bar chart.
pub const BANDS: usize = 96;

/// Per-band Goertzel coefficients + smoothed magnitudes.
#[derive(Debug, Clone)]
pub struct Spectrum {
    coeff: [f32; BANDS],
    level: [f32; BANDS],
}

impl Spectrum {
    /// Analyzer for the given sample rate.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let mut coeff = [0.0f32; BANDS];
        for (k, c) in coeff.iter_mut().enumerate() {
            // log spacing 50 Hz → 14 kHz across BANDS bands
            let f = 50.0 * (14_000.0f32 / 50.0).powf(k as f32 / (BANDS - 1) as f32);
            *c = 2.0 * (core::f32::consts::TAU * f / sample_rate).cos();
        }
        Self {
            coeff,
            level: [0.0; BANDS],
        }
    }

    /// Analyze one mono block; updates the smoothed band levels.
    pub fn analyze(&mut self, block: &[Sample]) {
        if block.is_empty() {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let norm = 2.0 / block.len() as f32;
        for k in 0..BANDS {
            let c = self.coeff[k];
            let (mut s1, mut s2) = (0.0f32, 0.0f32);
            for &x in block {
                let s0 = c.mul_add(s1, x - s2);
                s2 = s1;
                s1 = s0;
            }
            // Goertzel magnitude² at the block end
            let power = s2.mul_add(s2, s1.mul_add(s1, -(c * s1 * s2)));
            let mag = (power.max(0.0)).sqrt() * norm;
            // ballistics: pounce on rises, ring down on falls
            let l = &mut self.level[k];
            *l = if mag > *l {
                0.4f32.mul_add(*l, 0.6 * mag)
            } else {
                *l * 0.86
            };
        }
    }

    /// Smoothed band levels, `0..≈1`.
    #[must_use]
    pub fn levels(&self) -> [f32; BANDS] {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_sine(sp: &mut Spectrum, freq: f32, sr: f32, blocks: usize) {
        let mut n = 0u32;
        for _ in 0..blocks {
            let block: Vec<f32> = (0..128)
                .map(|_| {
                    n += 1;
                    (core::f32::consts::TAU * freq * n as f32 / sr).sin()
                })
                .collect();
            sp.analyze(&block);
        }
    }

    #[test]
    fn a_sine_lights_its_own_band_brightest() {
        let sr = 48_000.0;
        let mut sp = Spectrum::new(sr);
        // 440 Hz sits between bands; find the analyzer's nearest center
        feed_sine(&mut sp, 440.0, sr, 60);
        let levels = sp.levels();
        let brightest = levels
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap();
        // 440 Hz ≈ band index round(ln(440/50)/ln(14000/50) * 95) = 37
        assert!(
            (34..=40).contains(&brightest),
            "440 Hz lit band {brightest}, levels {levels:?}"
        );
        assert!(levels[brightest] > 0.2);
    }

    #[test]
    fn silence_decays_to_black() {
        let sr = 48_000.0;
        let mut sp = Spectrum::new(sr);
        feed_sine(&mut sp, 1000.0, sr, 40);
        let silent = [0.0f32; 128];
        for _ in 0..400 {
            sp.analyze(&silent);
        }
        assert!(sp.levels().iter().all(|&l| l < 1e-3));
    }
}
