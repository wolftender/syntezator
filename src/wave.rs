pub trait Wave {
    // Returned value in [-1.0; 1.0]
    fn value(&self, frequency: f32, time: f32) -> f32;
}

#[derive(Debug, Clone, Copy)]
pub struct SineWave;

impl Wave for SineWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        (core::f32::consts::TAU * frequency * time).sin()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SquareWave;

impl Wave for SquareWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        if (time * frequency).fract() < 0.5 {
            1.0
        } else {
            -1.0
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SawtoothWave;

impl Wave for SawtoothWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        2.0 * (time * frequency - (time * frequency + 0.5).floor())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriangleWave;

impl Wave for TriangleWave {
    fn value(&self, frequency: f32, time: f32) -> f32 {
        2.0 * (2.0 * (time * frequency - (time * frequency + 0.5).floor())).abs() - 1.0
    }
}
