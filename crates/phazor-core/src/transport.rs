//! The transport: musical time derived from a frame counter, nothing else.

/// Position and tempo state. The single source of musical truth.
///
/// Invariant: `frame` only advances inside `Engine::process`, by exactly the
/// number of frames rendered. Beats are *derived* from frames on demand, so
/// there is no accumulating float drift between the two clocks.
#[derive(Debug, Clone, Copy)]
pub struct Transport {
    /// Output sample rate in Hz. Fixed at construction (Web Audio contexts
    /// never change rate mid-life).
    sample_rate: f64,
    /// Frames rendered since the transport last started from zero.
    frame: u64,
    /// Tempo in beats per minute.
    bpm: f64,
    /// Whether the musical clock advances.
    playing: bool,
}

impl Transport {
    /// A stopped transport at frame zero.
    #[must_use]
    pub fn new(sample_rate: f64, bpm: f64) -> Self {
        Self {
            sample_rate,
            frame: 0,
            bpm,
            playing: false,
        }
    }

    /// Output sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Current tempo in BPM.
    #[must_use]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Set tempo. Takes effect at the next block; beat position is preserved
    /// by rebasing the frame counter so `beats()` is continuous across the
    /// change (no playhead jump).
    pub fn set_bpm(&mut self, bpm: f64) {
        let beats_now = self.beats();
        self.bpm = bpm.clamp(20.0, 999.0);
        self.frame = self.beats_to_frames(beats_now);
    }

    /// Whether the musical clock is advancing.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Start the clock (resumes from the current position).
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Stop and rewind to zero. (Pause = `playing = false` without rewind is
    /// deliberately not offered yet: two-state transport keeps the sequencer
    /// edge cases enumerable. Revisit when arrangement view exists.)
    pub fn stop(&mut self) {
        self.playing = false;
        self.frame = 0;
    }

    /// Frames since start.
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Musical position in beats.
    #[must_use]
    pub fn beats(&self) -> f64 {
        self.frames_to_beats(self.frame)
    }

    /// Convert an absolute frame count to beats at the current tempo.
    #[must_use]
    pub fn frames_to_beats(&self, frame: u64) -> f64 {
        frame as f64 / self.sample_rate * (self.bpm / 60.0)
    }

    /// Jump to a musical position (a session resuming where it left off).
    /// Frame-quantized; callers release voices first if notes are ringing.
    pub fn seek_beats(&mut self, beats: f64) {
        self.frame = self.beats_to_frames(beats.max(0.0));
    }

    /// Convert beats to frames at the current tempo (rounded to nearest).
    #[must_use]
    pub fn beats_to_frames(&self, beats: f64) -> u64 {
        (beats * 60.0 / self.bpm * self.sample_rate).round() as u64
    }

    /// Advance by `n` rendered frames. Only `Engine::process` calls this.
    pub(crate) fn advance(&mut self, n: usize) {
        if self.playing {
            self.frame += n as u64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn beats_frames_roundtrip() {
        let t = Transport::new(48_000.0, 120.0);
        for beats in [0.0, 0.25, 1.0, 3.75, 16.0, 1000.0] {
            let rt = t.frames_to_beats(t.beats_to_frames(beats));
            assert_relative_eq!(rt, beats, epsilon = 1e-6);
        }
    }

    #[test]
    fn tempo_change_preserves_position() {
        let mut t = Transport::new(48_000.0, 120.0);
        t.play();
        t.advance(48_000); // 1s @ 120bpm = 2 beats
        assert_relative_eq!(t.beats(), 2.0, epsilon = 1e-9);
        t.set_bpm(60.0);
        assert_relative_eq!(t.beats(), 2.0, epsilon = 1e-6);
    }

    #[test]
    fn stopped_transport_does_not_advance() {
        let mut t = Transport::new(48_000.0, 120.0);
        t.advance(128);
        assert_eq!(t.frame(), 0);
    }
}
