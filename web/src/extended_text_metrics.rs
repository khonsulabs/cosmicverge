#![allow(clippy::needless_pass_by_value)] // False positive triggered by #[wasm_bindgen]

use wasm_bindgen::{prelude::*, JsCast};

#[wasm_bindgen]
extern "C" {
    pub type ExtendedTextMetrics;

    #[wasm_bindgen(method, getter, js_name = actualBoundingBoxAscent)]
    pub fn actual_bounding_box_ascent(this: &ExtendedTextMetrics) -> f64;

    #[wasm_bindgen(method, getter, js_name = actualBoundingBoxDescent)]
    pub fn actual_bounding_box_descent(this: &ExtendedTextMetrics) -> f64;

    #[wasm_bindgen(method, getter, js_name = actualBoundingBoxLeft)]
    pub fn actual_bounding_box_left(this: &ExtendedTextMetrics) -> f64;

    #[wasm_bindgen(method, getter, js_name = actualBoundingBoxRight)]
    pub fn actual_bounding_box_right(this: &ExtendedTextMetrics) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn width(this: &ExtendedTextMetrics) -> f64;
}

impl From<web_sys::TextMetrics> for ExtendedTextMetrics {
    fn from(tm: web_sys::TextMetrics) -> Self {
        tm.unchecked_into()
    }
}

impl ExtendedTextMetrics {
    pub fn height(&self) -> f64 {
        self.actual_bounding_box_ascent() + self.actual_bounding_box_descent()
    }
}
