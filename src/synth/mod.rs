use std::{collections::HashSet, time::Duration};

use crate::midi::{MIDIEventKind, MIDIFileData, MetaEvent, Tempo};

#[allow(dead_code)]
pub mod raw;
pub mod web_audio;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
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

#[derive(Debug)]
struct MidiTrackMetadata {
    /// Stores channel numbers. The index in this vector represents the continuous channel index
    channel_idx: Vec<u8>,
    duration: Duration,
}

impl MidiTrackMetadata {
    fn new(channel_idx: Vec<u8>, duration: Duration) -> Self {
        Self {
            channel_idx,
            duration,
        }
    }

    fn channel_index(&self, channel: u8) -> usize {
        self.channel_idx
            .iter()
            .position(|&ch| ch == channel)
            .expect("channel is not part of this track")
    }
}

#[derive(Debug)]
pub struct MidiMetadata {
    tracks: Vec<MidiTrackMetadata>,
}

impl MidiMetadata {
    pub fn new(data: &MIDIFileData) -> Self {
        let mut tracks = vec![];
        for track in data.tracks() {
            let mut tick_duration = data.time_division().tick_duration(Tempo::default());

            let mut channels = HashSet::new();
            let mut duration = Duration::from_secs(0);

            for event in track.events() {
                duration += tick_duration * event.delta_time();

                match event.kind() {
                    MIDIEventKind::Channel(channel_event) => {
                        channels.insert(channel_event.channel());
                    }
                    MIDIEventKind::Meta(MetaEvent::EndOfTrack) => break,
                    MIDIEventKind::Meta(MetaEvent::SetTempo { tempo }) => {
                        tick_duration = data.time_division().tick_duration(*tempo);
                    }
                    MIDIEventKind::Meta(MetaEvent::CopyrightNotice { .. })
                    | MIDIEventKind::Meta(MetaEvent::SequenceTrackName { .. })
                    | MIDIEventKind::Meta(MetaEvent::InstrumentName { .. })
                    | MIDIEventKind::Meta(MetaEvent::Lyrics { .. })
                    | MIDIEventKind::Meta(MetaEvent::Marker { .. })
                    | MIDIEventKind::Meta(MetaEvent::CuePoint { .. }) => {
                        // Ignored
                    }
                    MIDIEventKind::Meta(_) => {
                        log::warn!("Unhandled meta in meta collection event: {event:?}")
                    }
                }
            }

            tracks.push(MidiTrackMetadata::new(
                channels.into_iter().collect(),
                duration,
            ));
        }

        Self { tracks }
    }

    pub fn total_duration(&self) -> Duration {
        self.tracks
            .iter()
            .map(|track| track.duration)
            .max()
            .unwrap_or_default()
    }
}
