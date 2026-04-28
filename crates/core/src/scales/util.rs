//! Tick-nicing helpers shared across scale impls.
//!
//! The algorithm follows d3-scale: pick a step from {1, 2, 5} × 10^k that
//! produces close to `count` ticks over the given span.

pub(super) fn nice_step(span: f64, count: usize) -> f64 {
    if span <= 0.0 || count == 0 {
        return 1.0;
    }
    let target = span / count as f64;
    let exp = target.log10().floor();
    let pow = 10f64.powf(exp);
    let err = target / pow;
    let mul = if err >= 7.5 {
        10.0
    } else if err >= 3.5 {
        5.0
    } else if err >= 1.5 {
        2.0
    } else {
        1.0
    };
    mul * pow
}

pub(super) fn format_tick(value: f64, step: f64) -> String {
    // Choose precision so the tick label can resolve `step` without trailing noise.
    let precision = (-step.log10().floor()).max(0.0) as usize;
    format!("{value:.precision$}")
}

pub(super) fn hash_f64(h: &mut u64, v: f64) {
    *h ^= v.to_bits();
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

pub(super) fn hash_f32(h: &mut u64, v: f32) {
    *h ^= u64::from(v.to_bits());
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

pub(super) fn hash_u32(h: &mut u64, v: u32) {
    *h ^= u64::from(v);
    *h = h.wrapping_mul(0x0100_0000_01b3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_step_produces_reasonable_ticks() {
        assert_eq!(nice_step(10.0, 5), 2.0);
        assert_eq!(nice_step(100.0, 10), 10.0);
        assert_eq!(nice_step(1.0, 5), 0.2);
    }
}
