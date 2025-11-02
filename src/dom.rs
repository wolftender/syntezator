//! Handles to DOM elements in the HTML, and helper functions for interacting with JS.
use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;
use web_sys::{Document, FileReader, js_sys::Uint8Array};

use crate::midi;

#[allow(dead_code)]
pub struct MidiInput {
    element: web_sys::HtmlInputElement,
    midi_cb: Rc<RefCell<dyn FnMut(midi::MIDIFileData)>>,
    error_cb: Rc<RefCell<dyn FnMut(midi::MIDIFileError)>>,
}

impl MidiInput {
    pub fn new<F: FnMut(midi::MIDIFileData) + 'static, E: FnMut(midi::MIDIFileError) + 'static>(
        document: &Document,
        midi_cb: F,
        error_cb: E,
    ) -> Self {
        let element = document
            .get_element_by_id("midi")
            .expect("MIDI input element not found")
            .dyn_into::<web_sys::HtmlInputElement>()
            .expect("failed to cast midi input to HtmlInputElement");

        let midi_cb = Rc::new(RefCell::new(midi_cb));
        let error_cb = Rc::new(RefCell::new(error_cb));
        let midi_cb_c = midi_cb.clone();
        let error_cb_c = error_cb.clone();

        let on_change_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let input: web_sys::HtmlInputElement = event
                .target()
                .unwrap()
                .dyn_into()
                .expect("cannot get correct target for change");

            if let Some(file) = input.files().and_then(|f| f.item(0)) {
                let reader = FileReader::new().expect("failed to create file reader");
                let midi_cb_c = midi_cb_c.clone();
                let error_cb_c = error_cb_c.clone();

                let on_load_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                    let reader: web_sys::FileReader = event
                        .target()
                        .unwrap()
                        .dyn_into()
                        .expect("cannot get correct target for load");

                    let array_buffer = reader.result().expect("failed to get result");
                    let buffer = Uint8Array::new(&array_buffer).to_vec();

                    match midi::MIDIFileData::try_from(&buffer[..]) {
                        Ok(data) => (midi_cb_c.borrow_mut())(data),
                        Err(error) => (error_cb_c.borrow_mut())(error),
                    }
                }) as Box<dyn FnMut(_)>);

                reader.set_onload(Some(on_load_closure.as_ref().unchecked_ref()));
                reader
                    .read_as_array_buffer(&file)
                    .expect("cannot read as array buffer");

                on_load_closure.forget();
            }
        }) as Box<dyn FnMut(_)>);

        element
            .add_event_listener_with_callback("change", on_change_closure.as_ref().unchecked_ref())
            .expect("failed to set change event handler");
        on_change_closure.forget();

        Self {
            element,
            midi_cb,
            error_cb,
        }
    }
}

pub struct SynthKind {
    element: web_sys::HtmlSelectElement,
}

pub enum SynthKindOption {
    Raw,
    WebAudio,
}

impl SynthKind {
    pub fn new(document: &Document) -> Self {
        let element = document
            .get_element_by_id("synth-kind")
            .expect("synth-kind select element not found")
            .dyn_into::<web_sys::HtmlSelectElement>()
            .expect("failed to cast synth-kind to HtmlSelectElement");

        Self { element }
    }

    pub fn get_selected(&self) -> SynthKindOption {
        let value = self.element.value();
        match value.as_str() {
            "raw" => SynthKindOption::Raw,
            "web_audio" => SynthKindOption::WebAudio,
            _ => panic!("unknown synth kind selected"),
        }
    }
}

pub struct WaveKind {
    element: web_sys::HtmlSelectElement,
}

pub enum WaveKindOption {
    Sine,
    Square,
    Sawtooth,
    Triangle,
}

impl WaveKind {
    pub fn new(document: &Document) -> Self {
        let element = document
            .get_element_by_id("wave-kind")
            .expect("wave-kind select element not found")
            .dyn_into::<web_sys::HtmlSelectElement>()
            .expect("failed to cast wave-kind to HtmlSelectElement");

        Self { element }
    }

    pub fn get_selected(&self) -> WaveKindOption {
        let value = self.element.value();
        match value.as_str() {
            "sine" => WaveKindOption::Sine,
            "square" => WaveKindOption::Square,
            "sawtooth" => WaveKindOption::Sawtooth,
            "triangle" => WaveKindOption::Triangle,
            _ => panic!("unknown wave kind selected"),
        }
    }
}
