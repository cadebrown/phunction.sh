//! Telemetry the engine streams back to the UI, one POD frame per block.

/// A snapshot of engine state after one render block. `Copy`, fixed-size,
/// no heap — it rides a lock-free ring to the UI thread, where it drives
/// meters, the playhead, and the debug HUD.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterFrame {
    /// Absolute frame count at the *end* of the block.
    pub frame: u64,
    /// Musical position in beats at the end of the block.
    pub beats: f64,
    /// Peak absolute sample value in the block, left channel.
    pub peak_l: f32,
    /// Peak absolute sample value in the block, right channel.
    pub peak_r: f32,
    /// Root-mean-square level of the block, left channel.
    pub rms_l: f32,
    /// Root-mean-square level of the block, right channel.
    pub rms_r: f32,
    /// Number of voices currently sounding.
    pub voices: u8,
    /// True if the transport is running.
    pub playing: bool,
    /// 16-band smoothed spectrum (60 Hz → 12 kHz, log-spaced).
    pub bands: [f32; crate::spectrum::BANDS],
    /// Decimated mono waveform of the block (the oscilloscope's trace).
    pub scope: [f32; SCOPE],
}

/// Points per scope trace. 64 f32s keeps `MeterFrame` a small `Copy` POD
/// while giving the LCD enough resolution to show real waveshape.
pub const SCOPE: usize = 64;

// Hand-written because `Default` for arrays stops at 32 elements.
impl Default for MeterFrame {
    fn default() -> Self {
        Self {
            frame: 0,
            beats: 0.0,
            peak_l: 0.0,
            peak_r: 0.0,
            rms_l: 0.0,
            rms_r: 0.0,
            voices: 0,
            playing: false,
            bands: [0.0; crate::spectrum::BANDS],
            scope: [0.0; SCOPE],
        }
    }
}

/// Accumulates peak/RMS over one block.
#[derive(Debug, Default, Clone, Copy)]
pub struct BlockMeter {
    peak_l: f32,
    peak_r: f32,
    sum_sq_l: f64,
    sum_sq_r: f64,
    n: usize,
}

impl BlockMeter {
    /// Feed one stereo sample.
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) {
        self.peak_l = self.peak_l.max(l.abs());
        self.peak_r = self.peak_r.max(r.abs());
        self.sum_sq_l += f64::from(l) * f64::from(l);
        self.sum_sq_r += f64::from(r) * f64::from(r);
        self.n += 1;
    }

    /// Finish the block: produce levels and reset the accumulator.
    pub fn finish(&mut self) -> (f32, f32, f32, f32) {
        let n = self.n.max(1) as f64;
        let out = (
            self.peak_l,
            self.peak_r,
            (self.sum_sq_l / n).sqrt() as f32,
            (self.sum_sq_r / n).sqrt() as f32,
        );
        *self = Self::default();
        out
    }
}
