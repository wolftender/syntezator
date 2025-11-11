use log::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlCanvasElement};

pub struct BarPlotter {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl BarPlotter {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or(JsValue::from("failed to get canvas drawing context"))?
            .dyn_into()?;

        Ok(Self { canvas, context })
    }

    pub fn plot(&self, min: f32, max: f32, data: &[f32]) {
        let width = self.canvas.width();
        let height = self.canvas.height();
        let fwidth = f64::from(width);
        let fheight = f64::from(height);

        self.context.set_fill_style_str("#000000");
        self.context.fill_rect(0.0, 0.0, fwidth, fheight);

        let bar_width = fwidth / (data.len() as f64);
        let mut offset_x = 0.0;

        for sample in data {
            let sample = sample.clamp(min, max);
            let normalized = ((sample - min) / (max - min)) as f64;
            let bar_height = normalized * fheight * 0.75;
            let middle_y = fheight * 0.5;
            let hue = (normalized * 96.0).floor() as u32;

            self.context
                .set_fill_style_str(&format!("hsl({hue}, 100%, 50%)"));
            self.context.fill_rect(
                offset_x,
                middle_y - (0.5 * bar_height),
                bar_width,
                bar_height,
            );

            offset_x = offset_x + bar_width;
        }
    }
}

pub struct LinePlotter {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    first: usize,
    fill: usize,
    buffer: Vec<f32>,
}

impl LinePlotter {
    pub fn new(canvas: HtmlCanvasElement, num_samples: usize) -> Result<Self, JsValue> {
        let context = canvas
            .get_context("2d")?
            .ok_or(JsValue::from("failed to get canvas drawing context"))?
            .dyn_into()?;

        let buffer = vec![0.0; num_samples];

        Ok(Self {
            canvas,
            context,
            first: 0,
            fill: 0,
            buffer,
        })
    }

    fn internal_append(&mut self, data: &[f32]) {
        // append data to the internal circular buffer
        let capacity = self.buffer.len();
        for sample in data {
            if self.fill < capacity {
                let idx = (self.first + self.fill) % capacity;
                self.buffer[idx] = *sample;
                self.fill = self.fill + 1;
            } else {
                self.buffer[self.first] = *sample;
                self.first = (self.first + 1) % capacity;
            }
        }
    }

    pub fn plot(&mut self, min: f32, max: f32, data: &[f32]) {
        self.internal_append(data);

        let width = self.canvas.width();
        let height = self.canvas.height();
        let fwidth = f64::from(width);
        let fheight = f64::from(height);

        self.context.set_fill_style_str("#000000");
        self.context.fill_rect(0.0, 0.0, fwidth, fheight);

        let step_width = fwidth / (self.fill as f64);
        let mut offset_x = 0.0;

        self.context.set_stroke_style_str("#ffffff");
        self.context.begin_path();
        self.context.move_to(0.0, fheight * 0.5);

        for idx in 0..self.fill {
            let sample = self.buffer[(self.first + idx) % self.buffer.len()];
            let normalized = ((sample - min) / (max - min)) as f64;

            self.context.line_to(offset_x, fheight * normalized);
            offset_x = offset_x + step_width;
        }

        self.context.stroke();
    }
}

pub struct AudioVisualizer {
    canvas_freq: HtmlCanvasElement,
    canvas_time: HtmlCanvasElement,
    analyzer: AnalyserNode,
    freq_data: Vec<f32>,
    time_data: Vec<f32>,
    plotter_freq: BarPlotter,
    plotter_time: LinePlotter,
}

impl AudioVisualizer {
    pub fn analyzer_node(&self) -> &AnalyserNode {
        &self.analyzer
    }

    pub fn new(
        audio_context: AudioContext,
        canvas_freq: HtmlCanvasElement,
        canvas_time: HtmlCanvasElement,
    ) -> Result<Self, JsValue> {
        let analyzer = audio_context.create_analyser()?;
        analyzer.set_fft_size(128);

        let num_freq_bins = analyzer.frequency_bin_count();
        let num_fft_data = analyzer.fft_size();

        let freq_data = vec![0.0; num_freq_bins as usize];
        let time_data = vec![0.0; num_fft_data as usize];

        let plotter_freq = BarPlotter::new(canvas_freq.clone())?;
        let plotter_time = LinePlotter::new(canvas_time.clone(), 4096)?;

        info!("data len {}", freq_data.len());
        info!("data len f64 {}", freq_data.len() as f64);

        Ok(Self {
            analyzer,
            canvas_freq,
            canvas_time,
            freq_data,
            time_data,
            plotter_freq,
            plotter_time,
        })
    }

    pub fn redraw(&mut self) {
        // get data
        self.analyzer.get_float_frequency_data(&mut self.freq_data);
        self.analyzer
            .get_float_time_domain_data(&mut self.time_data);

        let min_db = self.analyzer.min_decibels();
        let max_db = self.analyzer.max_decibels();

        self.plotter_freq
            .plot(min_db as f32, max_db as f32, &self.freq_data);

        self.plotter_time.plot(-1.0, 1.0, &self.time_data);
    }
}
