use wasm_bindgen::prelude::*;
mod dom;

use crate::{midi::MIDIFileData, synth::MidiSynth};

#[allow(dead_code)]
mod midi;
mod synth;
#[allow(dead_code)]
mod wave;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let ctx = web_sys::AudioContext::new()?;

    let midi_bytes = include_bytes!("./assets/test.mid");
    let midi_data = MIDIFileData::try_from(&midi_bytes[..]).unwrap();
    let synth = MidiSynth::new(midi_data);
    let sample_rate = ctx.sample_rate();
    let (buffer_length, buffers) = synth.create_buffer(sample_rate as u32);

    let flattened_buffers = buffers.into_iter().flatten().collect::<Vec<_>>();

    let buffer = ctx.create_buffer(
        flattened_buffers.len() as u32,
        buffer_length as u32,
        sample_rate,
    )?;

    for channel in 0..buffer.number_of_channels() {
        buffer.copy_to_channel(&flattened_buffers[channel as usize], channel as i32)?;
    }

    let source = ctx.create_buffer_source()?;
    source.set_buffer(Some(&buffer));
    source.connect_with_audio_node(&ctx.destination())?;
    source.start()?;

    Ok(())
}
