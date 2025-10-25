//! Handles to DOM elements in the HTML, and helper functions for interacting with JS.
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{AudioBuffer, Document, js_sys};

pub struct MidiInput {
    element: web_sys::HtmlInputElement,
}

impl MidiInput {
    pub fn new(document: &Document) -> Self {
        let element = document
            .get_element_by_id("midi")
            .expect("MIDI input element not found")
            .dyn_into::<web_sys::HtmlInputElement>()
            .expect("failed to cast midi input to HtmlInputElement");

        Self { element }
    }

    pub fn listen_to_source_change(&self, callback: &dyn Fn(Vec<u8>)) {
        let this = self.element.clone();

        let closure = Closure::<dyn Fn(_)>::new(move |e: web_sys::InputEvent| {
            let Some(first) = this.files().and_then(|f| f.item(0)) else {
                // no file selected
                return;
            };

            spawn_local(async move {
                let bytes = wasm_bindgen_futures::JsFuture::from(first.bytes())
                    .await
                    .expect("could not get bytes");
                let bytes = js_sys::Uint8Array::new(&bytes).to_vec();

                callback(bytes);
                log::info!("Array buffer loaded: {bytes:?}");
            });
        });

        self.element
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .expect("failed to add midi file listener");

        closure.forget();
    }
}

pub struct AudioControl {
    element: web_sys::HtmlAudioElement,
    source: web_sys::HtmlSourceElement,
}

impl AudioControl {
    pub fn new(document: &Document) -> Self {
        let element = document
            .get_element_by_id("audio")
            .expect("audio control element not found")
            .dyn_into::<web_sys::HtmlAudioElement>()
            .expect("failed to cast audio control to HtmlAudioElement");

        let source = element
            .query_selector("source")
            .expect("bad query")
            .expect("failed to find audio source")
            .dyn_into::<web_sys::HtmlSourceElement>()
            .expect("failed to cast audio source to HtmlSourceElement");

        Self { element, source }
    }
}
