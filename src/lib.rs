use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{
    dom::{PlaybackControls, SynthKind, SynthKindOption, WaveKind, WaveKindOption},
    midi::MIDIFileData,
    synth::MidiMetadata,
    wave::{SawtoothWave, SineWave, SquareWave, TriangleWave, Wave},
};
mod dom;

#[allow(dead_code)]
mod midi;
mod synth;

#[allow(dead_code)]
mod wave;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

struct MidiPlayerState {
    audio_context: web_sys::AudioContext,
    audio_source: web_sys::AudioBufferSourceNode,
}

impl MidiPlayerState {
    pub fn new(audio_context: web_sys::AudioContext) -> Result<Self, JsValue> {
        let audio_source = audio_context.create_buffer_source()?;

        Ok(Self {
            audio_context,
            audio_source,
        })
    }

    pub fn set_buffer(
        &mut self,
        midi_data: MIDIFileData,
        synth_kind: SynthKindOption,
        wave_kind: WaveKindOption,
    ) -> Result<(), JsValue> {
        let wave: &dyn Wave = match wave_kind {
            WaveKindOption::Sine => &SineWave,
            WaveKindOption::Square => &SquareWave,
            WaveKindOption::Sawtooth => &SawtoothWave,
            WaveKindOption::Triangle => &TriangleWave,
        };

        match synth_kind {
            SynthKindOption::Raw => {
                let synth = synth::raw::MidiSynth::new(midi_data);
                let sample_rate = self.audio_context.sample_rate();
                let (buffer_length, buffers) = synth.create_buffer(sample_rate as u32, wave);

                let flattened_buffers = buffers.into_iter().flatten().collect::<Vec<_>>();

                let audio_buffer = self.audio_context.create_buffer(
                    flattened_buffers.len() as u32,
                    buffer_length as u32,
                    sample_rate,
                )?;

                for channel in 0..audio_buffer.number_of_channels() {
                    audio_buffer
                        .copy_to_channel(&flattened_buffers[channel as usize], channel as i32)?;
                }

                self.audio_source.disconnect()?;
                self.audio_source = self.audio_context.create_buffer_source()?;
                self.audio_source.set_buffer(Some(&audio_buffer));
                self.audio_source
                    .connect_with_audio_node(&self.audio_context.destination())?;
                self.audio_source.start()?;
            }
            SynthKindOption::WebAudio => {
                let synth = synth::web_audio::MidiSynth::new(midi_data);
                synth.schedule(&self.audio_context, wave, &self.audio_context.destination())?;
                // TODO: remove existing playback
            }
        }

        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let _body = document.body().expect("document should have a body");

    let audio_context = web_sys::AudioContext::new()?;
    let player_state = Rc::new(RefCell::new(MidiPlayerState::new(audio_context)?));
    let player_state_c = player_state.clone();

    let synth_kind = SynthKind::new(&document);
    let wave_kind = WaveKind::new(&document);

    let playback_controls = PlaybackControls::new(&document);
    playback_controls.on_play_pause(move |is_play| {
        // TODO: implement play/pause
    });
    playback_controls.on_position_change(move |position| {
        // TODO: implement seeking
    });

    let _midi = dom::MidiInput::new(
        &document,
        move |midi_data| {
            let duration = MidiMetadata::new(&midi_data).total_duration();
            playback_controls.set_duration(duration);

            log::info!("midi file uploaded! tracks: {}", midi_data.num_tracks());
            for track in midi_data.tracks() {
                log::info!("track with {} events", track.events().len())
            }

            if let Err(error) = player_state_c.borrow_mut().set_buffer(
                midi_data,
                synth_kind.get_selected(),
                wave_kind.get_selected(),
            ) {
                log::error!("invalid midi file supplied: {:?}", error);
                alert(&format!("invalid midi file supplied: {:?}", error));
            }
        },
        |error| {
            log::error!("invalid midi file supplied: {:?}", error);
            alert(&format!("invalid midi file supplied: {:?}", error));
        },
    );

    Ok(())
}
