use core::f32;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
    vec,
};

use crate::{
    midi::{ChannelEventKind, MIDIEventKind, MIDIFileData, MetaEvent, Tempo},
    wave::{SineWave, Wave},
};

#[derive(Debug)]
struct MidiTrackMeta {
    /// Stores channel numbers. The index in this vector represents the continuous channel index
    channel_idx: Vec<u8>,
    duration: Duration,
}

impl MidiTrackMeta {
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
struct MidiMeta {
    tracks: Vec<MidiTrackMeta>,
}

impl MidiMeta {
    fn new(data: &MIDIFileData) -> Self {
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

            tracks.push(MidiTrackMeta::new(channels.into_iter().collect(), duration));
        }

        Self { tracks }
    }

    fn total_duration(&self) -> Duration {
        self.tracks
            .iter()
            .map(|track| track.duration)
            .max()
            .unwrap_or_default()
    }
}

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

pub struct MidiSynth {
    data: MIDIFileData,
    meta: MidiMeta,
}

impl MidiSynth {
    pub fn new(data: MIDIFileData) -> Self {
        Self {
            meta: MidiMeta::new(&data),
            data,
        }
    }

    /// Create a vector per track per channel filled with values from -1 to 1.
    ///
    /// All individual buffers are of the same length, equal to the first tuple element.
    pub fn create_buffer(&self, sample_rate: u32) -> (usize, Vec<Vec<Vec<f32>>>) {
        let buffer_length =
            (sample_rate as f32 * self.meta.total_duration().as_secs_f32()).floor() as usize;

        let mut buffers = self
            .meta
            .tracks
            .iter()
            .map(|track| vec![vec![0.0f32; buffer_length]; track.channel_idx.len()])
            .collect::<Vec<Vec<Vec<f32>>>>();

        for (track_index, track) in self.data.tracks().iter().enumerate() {
            let mut sample_number = 0;
            let mut samples_per_tick = (sample_rate
                * self.data.time_division().tick_duration(Tempo::default()))
            .as_secs_f32();

            let mut active_notes = HashMap::<usize, HashSet<MidiNote>>::new();

            for event in track.events() {
                let sample_delta = (event.delta_time() as f32 * samples_per_tick) as usize;

                // Fill notes from sample_number to sample_number + sample_delta with the currently active notes
                {
                    for (channel_buffer_idx, notes) in &active_notes {
                        let buffer = &mut buffers[track_index][*channel_buffer_idx]
                            [sample_number..sample_number + sample_delta];

                        for (sample_num, sample) in buffer.iter_mut().enumerate() {
                            let freq = notes
                                .iter()
                                .map(|n| {
                                    SineWave::new(n.frequency()).value(
                                        (sample_number + sample_num) as f32 / sample_rate as f32,
                                    )
                                })
                                .sum::<f32>()
                                / (active_notes.len() as f32).max(1.0);

                            *sample = freq;
                        }
                    }
                }

                match event.kind() {
                    MIDIEventKind::Channel(channel_event) => {
                        let channel_buffer_idx =
                            self.meta.tracks[track_index].channel_index(channel_event.channel());

                        match channel_event.kind() {
                            ChannelEventKind::NoteOff {
                                note,
                                // TODO: support velocity
                                velocity: _,
                            } => {
                                if let Some(notes) = active_notes.get_mut(&channel_buffer_idx) {
                                    notes.remove(&MidiNote::new(*note));
                                }
                            }
                            ChannelEventKind::NoteOn {
                                note,
                                // TODO: support velocity
                                velocity: _,
                            } => {
                                let notes = active_notes.entry(channel_buffer_idx).or_default();
                                notes.insert(MidiNote::new(*note));
                            }
                            ChannelEventKind::NoteAftertouch { .. }
                            | ChannelEventKind::Controller { .. }
                            | ChannelEventKind::ProgramChange { .. }
                            | ChannelEventKind::ChannelAftertouch { .. }
                            | ChannelEventKind::PitchBend { .. } => {
                                log::warn!("Unhandled channel event: {channel_event:?}")
                            }
                        }
                    }
                    MIDIEventKind::Meta(MetaEvent::EndOfTrack) => break,
                    MIDIEventKind::Meta(MetaEvent::SetTempo { tempo }) => {
                        samples_per_tick = (sample_rate
                            * self.data.time_division().tick_duration(*tempo))
                        .as_secs_f32();
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
                        log::warn!("Unhandled meta in buffer creation event: {event:?}")
                    }
                }

                sample_number += sample_delta;
            }
        }

        (buffer_length, buffers)
    }
}
