use std::{
    f32::consts::{PI, TAU},
    sync::LazyLock,
};

pub trait Wave: core::fmt::Debug {
    /// Returned value in [-1.0; 1.0]
    fn value(&self, frequency: f32, time: f32) -> f32;

    /// A decomposition of the wave into sine and cosine components.
    /// The wave can be reconstructed with an inverse Fourier transform.
    /// See: https://webaudio.github.io/web-audio-api/#waveform-generation.
    ///
    /// Returned terms are of the same length.
    /// The sine terms are:
    /// - First: does nothing
    /// - Second: fundamental frequency
    /// - Rest: overtone frequencies
    ///
    /// The cosine terms are:
    /// - First: DC offset
    /// - Second: fundamental frequency
    /// - Rest: overtone frequencies
    fn decompose(&self) -> (&[f32], &[f32]);
}

#[derive(Debug, Clone, Copy)]
pub struct SineWave;

impl Wave for SineWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        let t = frequency * time;
        (TAU * t).sin()
    }

    fn decompose(&self) -> (&[f32], &[f32]) {
        // src: https://webaudio.github.io/web-audio-api/#oscillator-coefficients
        static REAL: [f32; 2] = [0.0, 0.0];
        static IMAG: [f32; 2] = [0.0, 1.0];
        (&REAL, &IMAG)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SquareWave;

impl Wave for SquareWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        let t = frequency * time;
        if t.fract() < 0.5 { 1.0 } else { -1.0 }
    }

    fn decompose(&self) -> (&[f32], &[f32]) {
        // src: https://webaudio.github.io/web-audio-api/#oscillator-coefficients
        static REAL: [f32; 4000] = [0.0; 4000];
        static IMAG: [f32; 4000] = const {
            let mut arr = [0.0; 4000];
            arr[0] = 0.0;
            let mut k = 1;
            while k < arr.len() {
                arr[k] = (2.0 / (k as f32 * PI)) * ((k % 2 * 2) as f32);
                k += 1;
            }
            arr
        };

        (&REAL, &IMAG)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SawtoothWave;

impl Wave for SawtoothWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        2.0 * (time * frequency - (time * frequency + 0.5).floor())
    }

    fn decompose(&self) -> (&[f32], &[f32]) {
        // src: https://webaudio.github.io/web-audio-api/#oscillator-coefficients
        static REAL: [f32; 4000] = [0.0; 4000];
        static IMAG: [f32; 4000] = const {
            let mut arr = [0.0; 4000];
            arr[0] = 0.0;
            let mut k = 1;
            while k < arr.len() {
                arr[k] = (1.0 - ((k + 1) % 2 * 2) as f32) * (2.0 / (k as f32 * PI));
                k += 1;
            }
            arr
        };

        (&REAL, &IMAG)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriangleWave;

impl Wave for TriangleWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        let t = frequency * time;

        2.0 * (2.0 * (t - (t + 0.5).floor())).abs() - 1.0
    }

    fn decompose(&self) -> (&[f32], &[f32]) {
        // src: https://webaudio.github.io/web-audio-api/#oscillator-coefficients
        static REAL: [f32; 4000] = [0.0; 4000];
        // sin is not const-fn :(
        static IMAG: LazyLock<[f32; 4000]> = LazyLock::new(|| {
            let mut arr = [0.0; 4000];
            arr[0] = 0.0;
            let mut k = 1;
            while k < arr.len() {
                arr[k] =
                    8.0 * (((k as f32 * PI) / 2.0).sin()) / ((k as f32 * PI) * (k as f32 * PI));
                k += 1;
            }
            arr
        });

        (&REAL, IMAG.as_ref())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CustomWave<'a> {
    real: &'a [f32],
    imag: &'a [f32],
}

impl<'a> CustomWave<'a> {
    pub fn new(real: &'a [f32], imag: &'a [f32]) -> Self {
        assert!(real.len() == imag.len());

        Self { real, imag }
    }
}

impl Wave for CustomWave<'_> {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        // slow inverse fourier transform
        let t = frequency * time;

        self.real
            .iter()
            .enumerate()
            .skip(1)
            .map(|(k, c)| c * (TAU * k as f32 * t).cos())
            .sum::<f32>()
            + self
                .imag
                .iter()
                .enumerate()
                .skip(1)
                .map(|(k, c)| c * (TAU * k as f32 * t).sin())
                .sum::<f32>()
    }

    fn decompose(&self) -> (&[f32], &[f32]) {
        (self.real, self.imag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decompose {
        use super::*;

        fn waves() -> Vec<Box<dyn Wave>> {
            vec![
                Box::new(SineWave),
                Box::new(SquareWave),
                Box::new(SawtoothWave),
                Box::new(TriangleWave),
            ]
        }

        #[test]
        #[ignore = "the results don't exactly align"]
        fn value_agrees_with_decomposition() {
            const EPS: f32 = 1e-3;

            let freqs = vec![220.0, 440.0, 880.0];

            for w in waves() {
                let (real, imag) = w.decompose();
                let custom_wave = CustomWave::new(real, imag);

                for &f in &freqs {
                    for t in (0..1000).map(|x| x as f32 / 1000.0) {
                        let v1 = w.value(f, t);
                        let v2 = custom_wave.value(f, t);

                        assert!(
                            (v1 - v2).abs() < EPS,
                            "Wave value and decomposition do not match for wave {:?}, frequency {}, time {}: {} vs {}",
                            w,
                            f,
                            t,
                            v1,
                            v2
                        );
                    }
                }
            }
        }

        #[test]
        fn same_length() {
            for w in waves() {
                let (real, imag) = w.decompose();
                assert_eq!(
                    real.len(),
                    imag.len(),
                    "Real and imaginary parts have different lengths for wave {:?}: {} vs {}",
                    w,
                    real.len(),
                    imag.len()
                );
            }
        }
    }
}
