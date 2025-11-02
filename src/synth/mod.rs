pub mod raw;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct MidiNote {
    note: u8,
}

impl MidiNote {
    fn new(note: u8) -> Self {
        Self { note }
    }

    fn frequency(&self) -> f32 {
        const A4_FREQUENCY: f32 = 440.0;
        const A4_MIDI_NOTE: f32 = 69.0;
        const NOTE_COUNT: f32 = 12.0;

        A4_FREQUENCY * 2.0f32.powf((self.note as f32 - A4_MIDI_NOTE) / NOTE_COUNT)
    }
}
