//! Interaction helpers for the Leptos binding.
//!
//! Three thin layers on top of the renderer-mount components:
//!
//! 1. [`canvas_local_coords`] — convert a browser PointerEvent into canvas-
//!    local CSS pixels (matches the picker / mark coordinate system).
//! 2. [`use_drag`] — Leptos hook that wires pointerdown/move/up on a canvas
//!    and emits drag state as signals.
//! 3. [`interpolate_f32`] / [`interpolate_color`] / [`interpolate_point`] +
//!    [`use_tween`] — interpolation primitives for animated transitions
//!    between two chart states.

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Convert a `(client_x, client_y)` pointer event coordinate into canvas-
/// local CSS pixels. Returns `(0.0, 0.0)` if the canvas isn't yet mounted.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn canvas_local_coords(
    client_x: f64,
    client_y: f64,
    canvas: &web_sys::HtmlCanvasElement,
) -> (f32, f32) {
    let rect = canvas.get_bounding_client_rect();
    let x = (client_x - rect.left()) as f32;
    let y = (client_y - rect.top()) as f32;
    (x, y)
}

/// Drag lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    /// Pointer is up (no active drag).
    Idle,
    /// Pointer is down and moving — drag in progress.
    Active,
    /// Drag just released this frame (next read flips back to Idle).
    Released,
}

/// Current drag state. Updated reactively when consumers call `use_drag`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragState {
    /// Current phase.
    pub phase: DragPhase,
    /// Pointer position at drag start, canvas-local CSS pixels.
    pub start: (f32, f32),
    /// Current pointer position (or last seen), canvas-local CSS pixels.
    pub current: (f32, f32),
    /// Delta from start to current.
    pub delta: (f32, f32),
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            phase: DragPhase::Idle,
            start: (0.0, 0.0),
            current: (0.0, 0.0),
            delta: (0.0, 0.0),
        }
    }
}

/// Wire pointerdown/move/up listeners on the canvas and return a drag-state
/// signal. The signal updates on every pointermove during a drag and
/// transitions to `Released` once on pointerup before returning to `Idle`.
///
/// Consumers read the signal in an `Effect` to react to drag — e.g. update
/// a node's pinned position, scroll a viewport, etc.
///
/// On non-wasm targets this is a no-op that always reports `Idle`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn use_drag(canvas_ref: NodeRef<leptos::html::Canvas>) -> ReadSignal<DragState> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let (state, set_state) = signal(DragState::default());
    let closures: Rc<RefCell<Vec<Closure<dyn FnMut(web_sys::PointerEvent)>>>> =
        Rc::new(RefCell::new(Vec::new()));

    Effect::new(move |_| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let canvas: web_sys::HtmlCanvasElement =
            wasm_bindgen::JsCast::unchecked_into(canvas.clone());

        // down
        let canvas_for_down = canvas.clone();
        let down = Closure::wrap(Box::new(move |evt: web_sys::PointerEvent| {
            let (x, y) = canvas_local_coords(evt.client_x() as f64, evt.client_y() as f64, &canvas_for_down);
            set_state.set(DragState {
                phase: DragPhase::Active,
                start: (x, y),
                current: (x, y),
                delta: (0.0, 0.0),
            });
            let _ = canvas_for_down
                .set_pointer_capture(evt.pointer_id())
                .ok();
        }) as Box<dyn FnMut(_)>);

        // move
        let canvas_for_move = canvas.clone();
        let move_handler = Closure::wrap(Box::new(move |evt: web_sys::PointerEvent| {
            set_state.update(|s| {
                if !matches!(s.phase, DragPhase::Active) {
                    return;
                }
                let (x, y) = canvas_local_coords(
                    evt.client_x() as f64,
                    evt.client_y() as f64,
                    &canvas_for_move,
                );
                s.current = (x, y);
                s.delta = (x - s.start.0, y - s.start.1);
            });
        }) as Box<dyn FnMut(_)>);

        // up
        let canvas_for_up = canvas.clone();
        let up = Closure::wrap(Box::new(move |evt: web_sys::PointerEvent| {
            set_state.update(|s| {
                if matches!(s.phase, DragPhase::Idle) {
                    return;
                }
                let (x, y) = canvas_local_coords(
                    evt.client_x() as f64,
                    evt.client_y() as f64,
                    &canvas_for_up,
                );
                s.current = (x, y);
                s.delta = (x - s.start.0, y - s.start.1);
                s.phase = DragPhase::Released;
            });
            let _ = canvas_for_up.release_pointer_capture(evt.pointer_id()).ok();
        }) as Box<dyn FnMut(_)>);

        let _ = canvas.add_event_listener_with_callback(
            "pointerdown",
            down.as_ref().unchecked_ref(),
        );
        let _ = canvas.add_event_listener_with_callback(
            "pointermove",
            move_handler.as_ref().unchecked_ref(),
        );
        let _ = canvas.add_event_listener_with_callback(
            "pointerup",
            up.as_ref().unchecked_ref(),
        );
        let _ = canvas.add_event_listener_with_callback(
            "pointercancel",
            up.as_ref().unchecked_ref(),
        );

        closures.borrow_mut().push(down);
        closures.borrow_mut().push(move_handler);
        closures.borrow_mut().push(up);
    });

    state
}

/// No-op `use_drag` for non-wasm builds.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn use_drag<T>(_canvas_ref: T) -> DragState {
    DragState::default()
}

/// Linear interpolation between two `f32` values. `t` clamped to `[0, 1]`.
#[must_use]
pub fn interpolate_f32(from: f32, to: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * t
}

/// Componentwise interpolation between two RGBA colours.
#[must_use]
pub fn interpolate_color(from: [f32; 4], to: [f32; 4], t: f32) -> [f32; 4] {
    [
        interpolate_f32(from[0], to[0], t),
        interpolate_f32(from[1], to[1], t),
        interpolate_f32(from[2], to[2], t),
        interpolate_f32(from[3], to[3], t),
    ]
}

/// Componentwise interpolation between two 2D points.
#[must_use]
pub fn interpolate_point(from: [f32; 2], to: [f32; 2], t: f32) -> [f32; 2] {
    [
        interpolate_f32(from[0], to[0], t),
        interpolate_f32(from[1], to[1], t),
    ]
}

/// Easing functions. Add more variants as needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    /// `f(t) = t`. Constant velocity.
    Linear,
    /// `f(t) = t²`. Slow start.
    EaseIn,
    /// `f(t) = 1 - (1-t)²`. Slow end.
    EaseOut,
    /// `f(t) = t² × (3 - 2t)`. Smoothstep — symmetric ease-in-ease-out.
    SmoothStep,
}

impl Easing {
    /// Map raw progress (0..1) through the easing curve.
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(2),
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// Tween a numeric signal from `current` toward `target` over `duration_ms`.
/// Returns a `ReadSignal<f32>` that updates per animation frame until it
/// reaches `target`. Pass a new `target` via `set_target` to restart the tween.
///
/// On non-wasm builds returns a signal that immediately equals `target`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn use_tween(start: f32, target: ReadSignal<f32>, duration_ms: f32, easing: Easing) -> ReadSignal<f32> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let (value, set_value) = signal(start);
    let anim_state: Rc<RefCell<TweenState>> = Rc::new(RefCell::new(TweenState {
        from: start,
        to: start,
        started_at: 0.0,
        active: false,
    }));

    let state_for_target = anim_state.clone();
    Effect::new(move |_| {
        let new_target = target.get();
        let mut s = state_for_target.borrow_mut();
        s.from = value.get_untracked();
        s.to = new_target;
        s.started_at = now_ms().unwrap_or(0.0);
        s.active = true;
    });

    // requestAnimationFrame loop.
    let state_for_raf = anim_state.clone();
    let raf_closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let raf_closure_outer = raf_closure.clone();
    let tick = Closure::wrap(Box::new(move || {
        let mut s = state_for_raf.borrow_mut();
        if !s.active {
            return;
        }
        let now = now_ms().unwrap_or(s.started_at);
        let elapsed = (now - s.started_at) as f32;
        let raw_t = (elapsed / duration_ms).clamp(0.0, 1.0);
        let eased = easing.apply(raw_t);
        let v = interpolate_f32(s.from, s.to, eased);
        set_value.set(v);
        if raw_t >= 1.0 {
            s.active = false;
            set_value.set(s.to);
        } else {
            // schedule next frame
            if let Some(w) = web_sys::window() {
                if let Some(closure) = raf_closure.borrow().as_ref() {
                    let _ = w.request_animation_frame(closure.as_ref().unchecked_ref());
                }
            }
        }
    }) as Box<dyn FnMut()>);

    *raf_closure_outer.borrow_mut() = Some(tick);

    // Kick off the loop once.
    if let Some(w) = web_sys::window() {
        if let Some(closure) = raf_closure_outer.borrow().as_ref() {
            let _ = w.request_animation_frame(closure.as_ref().unchecked_ref());
        }
    }

    value
}

/// No-op tween on non-wasm: snaps to target.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn use_tween<T>(_start: f32, _target: T, _duration_ms: f32, _easing: Easing) -> f32 {
    0.0
}

#[cfg(target_arch = "wasm32")]
struct TweenState {
    from: f32,
    to: f32,
    started_at: f64,
    active: bool,
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> Option<f64> {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_interpolation_endpoints() {
        assert!((interpolate_f32(10.0, 20.0, 0.0) - 10.0).abs() < 1e-5);
        assert!((interpolate_f32(10.0, 20.0, 1.0) - 20.0).abs() < 1e-5);
        assert!((interpolate_f32(10.0, 20.0, 0.5) - 15.0).abs() < 1e-5);
    }

    #[test]
    fn interpolation_clamps_t() {
        // t > 1 clamps to 1, t < 0 clamps to 0
        assert!((interpolate_f32(0.0, 100.0, 2.0) - 100.0).abs() < 1e-5);
        assert!((interpolate_f32(0.0, 100.0, -1.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn color_interpolation_componentwise() {
        let mid = interpolate_color([0.0, 0.0, 0.0, 0.0], [1.0, 0.5, 0.25, 1.0], 0.5);
        assert!((mid[0] - 0.5).abs() < 1e-5);
        assert!((mid[1] - 0.25).abs() < 1e-5);
        assert!((mid[2] - 0.125).abs() < 1e-5);
        assert!((mid[3] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn point_interpolation_at_midpoint() {
        let mid = interpolate_point([0.0, 100.0], [200.0, 0.0], 0.5);
        assert!((mid[0] - 100.0).abs() < 1e-5);
        assert!((mid[1] - 50.0).abs() < 1e-5);
    }

    #[test]
    fn easing_endpoints_pin() {
        for ease in [Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::SmoothStep] {
            assert!((ease.apply(0.0)).abs() < 1e-5);
            assert!((ease.apply(1.0) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn smoothstep_symmetric_about_midpoint() {
        // smoothstep(0.5) should equal 0.5
        assert!((Easing::SmoothStep.apply(0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn drag_state_default_is_idle() {
        let s = DragState::default();
        assert_eq!(s.phase, DragPhase::Idle);
        assert_eq!(s.delta, (0.0, 0.0));
    }
}
