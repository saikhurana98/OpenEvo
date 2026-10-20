//! Presentation only: fast attack, brief hold and smooth release. Never alters audio.
#[derive(Clone, Copy, Default)]
pub struct Envelope {
    value: f32,
    hold: f32,
}
impl Envelope {
    pub fn value(&self) -> f32 { self.value }
    pub fn reset(&mut self) { *self = Self::default(); }
    pub fn advance(&mut self, target: f32, elapsed: f32) -> f32 {
        let target = if target.is_finite() { target.clamp(0.0, 1.0) } else { 0.0 };
        let elapsed = elapsed.clamp(0.0, 0.25);
        if target >= self.value {
            self.value += (target - self.value) * (1.0 - (-elapsed / 0.012).exp());
            self.hold = 0.065;
        } else {
            let release_time = (elapsed - self.hold).max(0.0);
            self.hold = (self.hold - elapsed).max(0.0);
            self.value += (target - self.value) * (1.0 - (-release_time / 0.22).exp());
        }
        if self.value < 0.0005 { self.value = 0.0; }
        self.value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transient_attacks_quickly_then_holds_and_releases_monotonically() {
        let mut meter = Envelope::default();
        let peak = meter.advance(1.0, 0.033);
        assert!(peak > 0.93 && peak < 1.0);
        assert_eq!(meter.advance(0.0, 0.033), peak);
        let mut previous = peak;
        for _ in 0..90 {
            let value = meter.advance(0.0, 1.0 / 60.0);
            assert!(value <= previous && value >= 0.0);
            previous = value;
        }
        assert!(previous < 0.002);
    }
    #[test]
    fn release_is_independent_of_frame_rate() {
        let mut a = Envelope::default();
        a.advance(1.0, 0.25);
        let mut b = a;
        for _ in 0..30 { a.advance(0.0, 1.0 / 30.0); }
        for _ in 0..60 { b.advance(0.0, 1.0 / 60.0); }
        assert!((a.value() - b.value()).abs() < 0.0001);
    }
    #[test]
    fn invalid_samples_are_bounded_and_disconnect_clears_immediately() {
        let mut meter = Envelope::default();
        assert!(meter.advance(5.0, 0.05) <= 1.0);
        assert!(meter.advance(f32::NAN, 0.02).is_finite());
        meter.reset();
        assert_eq!(meter.value(), 0.0);
    }
}
