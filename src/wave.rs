pub trait Wave {
    // Value in [-1.0; 1.0]
    fn value(&self, time: f32) -> f32;

    fn frequency(&self) -> f32;
}

#[derive(Debug, Clone, Copy)]
pub struct SineWave {
    frequency: f32,
}

impl SineWave {
    pub fn new(frequency: f32) -> Self {
        Self { frequency }
    }
}

impl Wave for SineWave {
    fn value(&self, time: f32) -> f32 {
        (core::f32::consts::TAU * self.frequency * time).sin()
    }

    fn frequency(&self) -> f32 {
        self.frequency
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SquareWave {
    frequency: f32,
}

impl SquareWave {
    pub fn new(frequency: f32) -> Self {
        Self { frequency }
    }
}

impl Wave for SquareWave {
    fn value(&self, time: f32) -> f32 {
        if (time * self.frequency).fract() < 0.5 {
            1.0
        } else {
            -1.0
        }
    }

    fn frequency(&self) -> f32 {
        self.frequency
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SawtoothWave {
    frequency: f32,
}

impl SawtoothWave {
    pub fn new(frequency: f32) -> Self {
        Self { frequency }
    }
}

impl Wave for SawtoothWave {
    fn value(&self, time: f32) -> f32 {
        2.0 * (time * self.frequency - (time * self.frequency + 0.5).floor())
    }

    fn frequency(&self) -> f32 {
        self.frequency
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriangleWave {
    frequency: f32,
}

impl TriangleWave {
    pub fn new(frequency: f32) -> Self {
        Self { frequency }
    }
}

impl Wave for TriangleWave {
    fn value(&self, time: f32) -> f32 {
        2.0 * (2.0 * (time * self.frequency - (time * self.frequency + 0.5).floor())).abs() - 1.0
    }

    fn frequency(&self) -> f32 {
        self.frequency
    }
}
