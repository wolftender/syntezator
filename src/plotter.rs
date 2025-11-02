use log::info;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    AnalyserNode, AudioContext, CanvasRenderingContext2d, HtmlCanvasElement, console::info,
};

pub struct Plotter {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl Plotter {
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
        let half_height = fheight * 0.5;

        self.context.set_fill_style_str("#000000");
        self.context.fill_rect(0.0, 0.0, fwidth, fheight);

        let bar_width = fwidth / (data.len() as f64);
        let mut offset_x = 0.0;

        for sample in data {
            let normalized = ((sample - min) / (max - min)) as f64;
            let bar_height = normalized * fheight * 100.0;

            //self.context
            //    .set_fill_style_str(&format!("rgb({}, 50, 50)", (bar_height * 255.0).floor()));
            self.context.set_fill_style_str("#ff0000");
            self.context.fill_rect(
                offset_x,
                half_height - (0.5 * bar_height),
                bar_width,
                bar_height,
            );

            offset_x = offset_x + bar_width;
        }
    }
}

pub struct AudioVisualizer {
    canvas_freq: HtmlCanvasElement,
    canvas_time: HtmlCanvasElement,
    analyzer: AnalyserNode,
    freq_data: Vec<f32>,
    time_data: Vec<f32>,
    plotter_freq: Plotter,
    plotter_time: Plotter,
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

        log::info!("AAAAAA");
        analyzer.set_fft_size(128);

        let num_freq_bins = analyzer.frequency_bin_count();
        let num_fft_data = analyzer.fft_size();

        let freq_data = vec![0.0; num_freq_bins as usize];
        let time_data = vec![0.0; num_fft_data as usize];

        let plotter_freq = Plotter::new(canvas_freq.clone())?;
        let plotter_time = Plotter::new(canvas_time.clone())?;

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

    fn resize_if_needed(&mut self) {
        let num_freq_bins = self.analyzer.frequency_bin_count();
        let num_fft_data = self.analyzer.fft_size();

        if (num_freq_bins as usize) != self.freq_data.len() {
            info!("resize to {}", num_freq_bins);
            self.freq_data = vec![0.0; num_freq_bins as usize];
        }

        if (num_fft_data as usize) != self.time_data.len() {
            info!("resize to {}", num_fft_data);
            self.time_data = vec![0.0; num_fft_data as usize];
        }
    }

    pub fn redraw(&mut self) {
        self.resize_if_needed();

        // get data
        self.analyzer
            .get_float_frequency_data(&mut self.freq_data[..]);
        self.analyzer
            .get_float_time_domain_data(&mut self.freq_data[..]);

        let min_db = self.analyzer.min_decibels();
        let max_db = self.analyzer.max_decibels();

        log::info!("min db = {min_db}; max db = {max_db}");

        self.plotter_freq
            .plot(min_db as f32, max_db as f32, &self.freq_data);

        self.plotter_time
            .plot(min_db as f32, max_db as f32, &self.time_data);
    }
}
