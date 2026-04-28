//! Small DOM event helpers shared by overlay and interaction layers.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
pub fn event_target_value_as_f32(ev: &web_sys::Event, fallback: f32) -> f32 {
    let Some(target) = ev.target() else {
        return fallback;
    };
    let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
        return fallback;
    };
    input.value().parse().unwrap_or(fallback)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn event_target_value_as_f32(_ev: &web_sys::Event, fallback: f32) -> f32 {
    fallback
}

#[cfg(target_arch = "wasm32")]
pub fn event_offset_in_current_target(ev: &web_sys::MouseEvent) -> (f32, f32) {
    let Some(target) = ev.current_target() else {
        return (ev.offset_x() as f32, ev.offset_y() as f32);
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return (ev.offset_x() as f32, ev.offset_y() as f32);
    };
    let rect = element.get_bounding_client_rect();
    (
        ev.client_x() as f32 - rect.left() as f32,
        ev.client_y() as f32 - rect.top() as f32,
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn event_offset_in_current_target(ev: &web_sys::MouseEvent) -> (f32, f32) {
    (ev.offset_x() as f32, ev.offset_y() as f32)
}

#[cfg(target_arch = "wasm32")]
pub fn event_target_has_class(ev: &web_sys::MouseEvent, class_name: &str) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(mut element) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };

    loop {
        if element
            .class_name()
            .split_whitespace()
            .any(|name| name == class_name)
        {
            return true;
        }
        let Some(parent) = element.parent_element() else {
            return false;
        };
        element = parent;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn event_target_has_class(_ev: &web_sys::MouseEvent, _class_name: &str) -> bool {
    false
}
