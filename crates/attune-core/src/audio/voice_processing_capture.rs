mod buffered;
mod ducking;
mod streaming;

pub use buffered::VoiceProcessingCapture;
pub use streaming::VoiceProcessingMicCapture;

pub const VPIO_SAMPLE_RATE_HZ: f64 = 16_000.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn instantiate_and_drop() {
        let cap = VoiceProcessingCapture::new().expect("VPIO new failed");
        assert_eq!(cap.sample_rate(), VPIO_SAMPLE_RATE_HZ);
    }
}
