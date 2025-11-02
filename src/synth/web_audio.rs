use core::f32;
use std::{collections::HashMap, time::Duration};

use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::{
    midi::{ChannelEventKind, MIDIEventKind, MIDIFileData, MetaEvent, Tempo},
    synth::MidiNote,
    wave::Wave,
};

pub struct MidiSynth {
    data: MIDIFileData,
}

impl MidiSynth {
    pub fn new(data: MIDIFileData) -> Self {
        Self { data }
    }

    pub fn schedule(
        &self,
        ctx: &web_sys::AudioContext,
        wave: &dyn Wave,
        destination: &web_sys::AudioNode,
    ) -> Result<(), JsValue> {
        let (real, imag) = wave.decompose();
        let periodic_wave_options = {
            let options = web_sys::PeriodicWaveOptions::new();
            options.set_real(&JsValue::from(js_sys::Float32Array::from(real)));
            options.set_imag(&JsValue::from(js_sys::Float32Array::from(imag)));
            options
        };
        let periodic_wave = web_sys::PeriodicWave::new_with_options(ctx, &periodic_wave_options)?;

        for track in self.data.tracks() {
            let mut time = Duration::from_secs_f64(ctx.current_time());
            let mut tick_duration = self.data.time_division().tick_duration(Tempo::default());

            struct PlayedNote {
                start_time: Duration,
                on_velocity: u8,
            }

            let mut played_notes = HashMap::<(u8, MidiNote), PlayedNote>::new();

            for event in track.events() {
                time += tick_duration * event.delta_time();

                match event.kind() {
                    MIDIEventKind::Channel(channel_event) => match channel_event.kind() {
                        ChannelEventKind::NoteOff {
                            note,
                            velocity: off_velocity,
                        } => {
                            let note = MidiNote::new(*note);
                            if let Some(played_note) =
                                played_notes.remove(&(channel_event.channel(), note))
                            {
                                Self::schedule_note(
                                    ctx,
                                    destination,
                                    &periodic_wave,
                                    note,
                                    played_note.on_velocity,
                                    *off_velocity,
                                    played_note.start_time,
                                    time - played_note.start_time,
                                )?;
                            }
                        }
                        ChannelEventKind::NoteOn { note, velocity } => {
                            played_notes.insert(
                                (channel_event.channel(), MidiNote::new(*note)),
                                PlayedNote {
                                    start_time: time,
                                    on_velocity: *velocity,
                                },
                            );
                        }
                        ChannelEventKind::NoteAftertouch { .. }
                        | ChannelEventKind::Controller { .. }
                        | ChannelEventKind::ProgramChange { .. }
                        | ChannelEventKind::ChannelAftertouch { .. }
                        | ChannelEventKind::PitchBend { .. } => {
                            log::warn!("Unhandled channel event: {channel_event:?}")
                        }
                    },
                    MIDIEventKind::Meta(MetaEvent::EndOfTrack) => break,
                    MIDIEventKind::Meta(MetaEvent::SetTempo { tempo }) => {
                        tick_duration = self.data.time_division().tick_duration(*tempo);
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
            }
        }

        Ok(())
    }

    fn schedule_note(
        ctx: &web_sys::AudioContext,
        destination: &web_sys::AudioNode,
        periodic_wave: &web_sys::PeriodicWave,
        note: MidiNote,
        on_velocity: u8,
        off_velocity: u8,
        start_time: Duration,
        duration: Duration,
    ) -> Result<(), JsValue> {
        let end_time = start_time + duration;
        let oscillator = web_sys::OscillatorNode::new(ctx)?;

        oscillator.set_periodic_wave(periodic_wave);
        oscillator.frequency().set_value(note.frequency());
        oscillator.start_with_when(start_time.as_secs_f64())?;
        oscillator.stop_with_when(end_time.as_secs_f64())?;

        let gain = web_sys::GainNode::new(ctx)?;
        // on_velocity used as volume and attack
        let on_frac = on_velocity as f32 / 127.0;
        let max_attack_time = Duration::from_millis(100);
        // harder velocity -> shorter attack
        let attack_duration =
            Duration::from_micros((max_attack_time.as_micros() as f32 * (1.0 - on_frac)) as u64)
                .min(duration / 3);
        gain.gain()
            .set_value_at_time(0.0, start_time.as_secs_f64())?;
        gain.gain().exponential_ramp_to_value_at_time(
            on_frac + 0.0001,
            (start_time + attack_duration).as_secs_f64(),
        )?;

        // off_velocity used as release
        let off_frac = off_velocity as f32 / 127.0;
        let max_release_time = Duration::from_millis(2000);
        // harder velocity -> shorter release
        let release_duration =
            Duration::from_micros((max_release_time.as_micros() as f32 * (1.0 - off_frac)) as u64)
                .min(duration / 2);
        gain.gain()
            .set_value_at_time(on_frac, (end_time - release_duration).as_secs_f64())?;
        gain.gain()
            .exponential_ramp_to_value_at_time(0.0001, end_time.as_secs_f64())?;

        oscillator.connect_with_audio_node(&gain)?;
        gain.connect_with_audio_node(destination)?;

        Ok(())
    }
}
