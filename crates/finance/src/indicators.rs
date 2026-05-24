//! Finance indicator computations.
//!
//! Pure functions over `&[f32]` close-price series. Output series are aligned
//! with input length — unfilled periods carry `f32::NAN`. Consumers ignore
//! NaN positions when rendering.

/// Simple moving average. `window` >= 1; window=1 returns the input.
#[must_use]
pub fn moving_average(values: &[f32], window: usize) -> Vec<f32> {
    let n = values.len();
    let mut out = vec![f32::NAN; n];
    if window == 0 || window > n {
        return out;
    }
    let mut sum = 0.0_f64;
    for i in 0..n {
        sum += values[i] as f64;
        if i + 1 >= window {
            if i >= window {
                sum -= values[i - window] as f64;
            }
            out[i] = (sum / window as f64) as f32;
        }
    }
    out
}

/// Exponentially-weighted moving average. `alpha = 2 / (window + 1)`.
/// First `window-1` positions return NaN to align with SMA conventions.
#[must_use]
pub fn exponential_moving_average(values: &[f32], window: usize) -> Vec<f32> {
    let n = values.len();
    let mut out = vec![f32::NAN; n];
    if window == 0 || window > n {
        return out;
    }
    let alpha = 2.0_f32 / (window as f32 + 1.0);
    // Seed with SMA at index window-1.
    let mut sum = 0.0_f32;
    for i in 0..window {
        sum += values[i];
    }
    let mut ema = sum / window as f32;
    out[window - 1] = ema;
    for i in window..n {
        ema = alpha * values[i] + (1.0 - alpha) * ema;
        out[i] = ema;
    }
    out
}

/// Bollinger bands: (middle SMA, upper = mid + k×σ, lower = mid - k×σ).
/// `k` typically 2.0.
#[must_use]
pub fn bollinger_bands(values: &[f32], window: usize, k: f32) -> BollingerBands {
    let n = values.len();
    let mid = moving_average(values, window);
    let mut upper = vec![f32::NAN; n];
    let mut lower = vec![f32::NAN; n];
    if window == 0 || window > n {
        return BollingerBands { mid, upper, lower };
    }
    for i in window - 1..n {
        let mean = mid[i];
        if !mean.is_finite() {
            continue;
        }
        let mut var = 0.0_f64;
        for j in i + 1 - window..=i {
            let d = values[j] as f64 - mean as f64;
            var += d * d;
        }
        let stddev = (var / window as f64).sqrt() as f32;
        upper[i] = mean + k * stddev;
        lower[i] = mean - k * stddev;
    }
    BollingerBands { mid, upper, lower }
}

/// Output of `bollinger_bands`.
#[derive(Debug, Clone, PartialEq)]
pub struct BollingerBands {
    /// Middle band — the SMA.
    pub mid: Vec<f32>,
    /// Upper band — `mid + k×σ`.
    pub upper: Vec<f32>,
    /// Lower band — `mid - k×σ`.
    pub lower: Vec<f32>,
}

/// Relative Strength Index (RSI). Standard 14-period default; returns NaN
/// for the first `window` positions.
#[must_use]
pub fn rsi(values: &[f32], window: usize) -> Vec<f32> {
    let n = values.len();
    let mut out = vec![f32::NAN; n];
    if window == 0 || window + 1 > n {
        return out;
    }
    let mut gain_sum = 0.0_f32;
    let mut loss_sum = 0.0_f32;
    for i in 1..=window {
        let diff = values[i] - values[i - 1];
        if diff >= 0.0 {
            gain_sum += diff;
        } else {
            loss_sum -= diff;
        }
    }
    let mut avg_gain = gain_sum / window as f32;
    let mut avg_loss = loss_sum / window as f32;
    out[window] = compute_rsi(avg_gain, avg_loss);
    for i in window + 1..n {
        let diff = values[i] - values[i - 1];
        let g = if diff > 0.0 { diff } else { 0.0 };
        let l = if diff < 0.0 { -diff } else { 0.0 };
        avg_gain = (avg_gain * (window as f32 - 1.0) + g) / window as f32;
        avg_loss = (avg_loss * (window as f32 - 1.0) + l) / window as f32;
        out[i] = compute_rsi(avg_gain, avg_loss);
    }
    out
}

fn compute_rsi(avg_gain: f32, avg_loss: f32) -> f32 {
    if avg_loss < f32::EPSILON {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - (100.0 / (1.0 + rs))
}

/// MACD output: the MACD line (fast EMA − slow EMA), the signal line (EMA of
/// MACD), and the histogram (MACD − signal).
#[derive(Debug, Clone, PartialEq)]
pub struct Macd {
    /// MACD line: `EMA(values, fast) - EMA(values, slow)`.
    pub macd: Vec<f32>,
    /// Signal line: `EMA(macd, signal)`.
    pub signal: Vec<f32>,
    /// Histogram: `macd - signal`. Useful for momentum visualisations.
    pub histogram: Vec<f32>,
}

/// Moving Average Convergence Divergence. Defaults: fast=12, slow=26, signal=9.
#[must_use]
pub fn macd(values: &[f32], fast: usize, slow: usize, signal: usize) -> Macd {
    let n = values.len();
    let fast_ema = exponential_moving_average(values, fast);
    let slow_ema = exponential_moving_average(values, slow);
    let mut line = vec![f32::NAN; n];
    for i in 0..n {
        if fast_ema[i].is_finite() && slow_ema[i].is_finite() {
            line[i] = fast_ema[i] - slow_ema[i];
        }
    }
    // signal line is EMA of macd line over the populated tail.
    let mut signal_line = vec![f32::NAN; n];
    let start = line.iter().position(|v| v.is_finite()).unwrap_or(n);
    if start + signal <= n {
        let tail: Vec<f32> = line[start..].to_vec();
        let tail_signal = exponential_moving_average(&tail, signal);
        for (i, &v) in tail_signal.iter().enumerate() {
            signal_line[start + i] = v;
        }
    }
    let histogram: Vec<f32> = line
        .iter()
        .zip(signal_line.iter())
        .map(|(m, s)| {
            if m.is_finite() && s.is_finite() {
                m - s
            } else {
                f32::NAN
            }
        })
        .collect();
    Macd {
        macd: line,
        signal: signal_line,
        histogram,
    }
}

/// Average True Range. Wilder smoothing of true range over a `window`.
/// `highs.len() == lows.len() == closes.len()`.
#[must_use]
pub fn atr(highs: &[f32], lows: &[f32], closes: &[f32], window: usize) -> Vec<f32> {
    let n = highs.len();
    let mut out = vec![f32::NAN; n];
    if window == 0 || window >= n || lows.len() != n || closes.len() != n {
        return out;
    }
    let mut tr = vec![0.0_f32; n];
    tr[0] = highs[0] - lows[0];
    for i in 1..n {
        let h_l = highs[i] - lows[i];
        let h_pc = (highs[i] - closes[i - 1]).abs();
        let l_pc = (lows[i] - closes[i - 1]).abs();
        tr[i] = h_l.max(h_pc).max(l_pc);
    }
    // First ATR = simple mean of first `window` TRs.
    let mut sum = 0.0_f32;
    for i in 0..window {
        sum += tr[i];
    }
    let mut atr_val = sum / window as f32;
    out[window - 1] = atr_val;
    // Wilder's smoothing for the rest.
    for i in window..n {
        atr_val = (atr_val * (window as f32 - 1.0) + tr[i]) / window as f32;
        out[i] = atr_val;
    }
    out
}

/// Volume-Weighted Average Price. Cumulative sum of price×volume divided by
/// cumulative volume. Typically reset per-session; callers slice the input by
/// session if they want intraday VWAP.
#[must_use]
pub fn vwap(prices: &[f32], volumes: &[f32]) -> Vec<f32> {
    let n = prices.len();
    let mut out = vec![f32::NAN; n];
    if volumes.len() != n {
        return out;
    }
    let mut cum_pv = 0.0_f64;
    let mut cum_v = 0.0_f64;
    for i in 0..n {
        if !prices[i].is_finite() || !volumes[i].is_finite() {
            out[i] = if cum_v > 0.0 {
                (cum_pv / cum_v) as f32
            } else {
                f32::NAN
            };
            continue;
        }
        cum_pv += prices[i] as f64 * volumes[i] as f64;
        cum_v += volumes[i] as f64;
        out[i] = if cum_v > 0.0 {
            (cum_pv / cum_v) as f32
        } else {
            f32::NAN
        };
    }
    out
}

/// Ichimoku Kinko Hyo output: five lines + the cloud (spans A and B).
#[derive(Debug, Clone, PartialEq)]
pub struct Ichimoku {
    /// Tenkan-sen (conversion line): `(highest_high + lowest_low) / 2` over `tenkan` periods.
    pub tenkan: Vec<f32>,
    /// Kijun-sen (base line): same calc over `kijun` periods.
    pub kijun: Vec<f32>,
    /// Senkou Span A (leading span A): `(tenkan + kijun) / 2`, shifted forward `kijun` periods.
    pub senkou_a: Vec<f32>,
    /// Senkou Span B (leading span B): mid-range over `senkou_b` periods, shifted forward `kijun` periods.
    pub senkou_b: Vec<f32>,
    /// Chikou Span (lagging close): close shifted back `kijun` periods.
    pub chikou: Vec<f32>,
}

/// Ichimoku Kinko Hyo. Standard parameters: tenkan=9, kijun=26, senkou_b=52.
#[must_use]
pub fn ichimoku(
    highs: &[f32],
    lows: &[f32],
    closes: &[f32],
    tenkan: usize,
    kijun: usize,
    senkou_b: usize,
) -> Ichimoku {
    let n = highs.len();
    let tenkan_line = midrange(highs, lows, tenkan);
    let kijun_line = midrange(highs, lows, kijun);
    let senkou_b_line = midrange(highs, lows, senkou_b);

    let mut senkou_a = vec![f32::NAN; n];
    let mut senkou_b_shifted = vec![f32::NAN; n];
    for i in 0..n {
        if i + kijun < n {
            if tenkan_line[i].is_finite() && kijun_line[i].is_finite() {
                senkou_a[i + kijun] = (tenkan_line[i] + kijun_line[i]) * 0.5;
            }
            if senkou_b_line[i].is_finite() {
                senkou_b_shifted[i + kijun] = senkou_b_line[i];
            }
        }
    }

    let mut chikou = vec![f32::NAN; n];
    for i in kijun..n {
        chikou[i - kijun] = closes[i];
    }

    Ichimoku {
        tenkan: tenkan_line,
        kijun: kijun_line,
        senkou_a,
        senkou_b: senkou_b_shifted,
        chikou,
    }
}

fn midrange(highs: &[f32], lows: &[f32], window: usize) -> Vec<f32> {
    let n = highs.len();
    let mut out = vec![f32::NAN; n];
    if window == 0 || window > n || lows.len() != n {
        return out;
    }
    for i in window - 1..n {
        let mut h = f32::NEG_INFINITY;
        let mut l = f32::INFINITY;
        for j in (i + 1 - window)..=i {
            if highs[j] > h {
                h = highs[j];
            }
            if lows[j] < l {
                l = lows[j];
            }
        }
        out[i] = (h + l) * 0.5;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_window_one_returns_input() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let s = moving_average(&v, 1);
        assert_eq!(s, v);
    }

    #[test]
    fn sma_window_three_smooths() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let s = moving_average(&v, 3);
        assert!(s[0].is_nan());
        assert!(s[1].is_nan());
        assert!((s[2] - 2.0).abs() < 1e-5);
        assert!((s[3] - 3.0).abs() < 1e-5);
        assert!((s[4] - 4.0).abs() < 1e-5);
    }

    #[test]
    fn ema_tracks_then_smooths() {
        let v = vec![10.0_f32; 20];
        let e = exponential_moving_average(&v, 5);
        for i in 4..20 {
            assert!((e[i] - 10.0).abs() < 1e-5);
        }
    }

    #[test]
    fn bollinger_bands_widen_with_volatility() {
        let v: Vec<f32> = (0..20)
            .map(|i| if i % 2 == 0 { 1.0 } else { 100.0 })
            .collect();
        let bb = bollinger_bands(&v, 5, 2.0);
        // Upper > mid > lower in the populated region.
        for i in 4..20 {
            assert!(bb.upper[i] > bb.mid[i]);
            assert!(bb.mid[i] > bb.lower[i]);
        }
    }

    #[test]
    fn macd_components_align() {
        let v: Vec<f32> = (1..=60).map(|i| i as f32).collect();
        let m = macd(&v, 12, 26, 9);
        assert_eq!(m.macd.len(), v.len());
        assert_eq!(m.signal.len(), v.len());
        assert_eq!(m.histogram.len(), v.len());
        // On an increasing series, MACD should be positive once populated.
        let last = m.macd[59];
        assert!(last.is_finite() && last > 0.0);
    }

    #[test]
    fn atr_positive_for_real_data() {
        let highs: Vec<f32> = (1..=20).map(|i| i as f32 + 0.5).collect();
        let lows: Vec<f32> = (1..=20).map(|i| i as f32 - 0.5).collect();
        let closes: Vec<f32> = (1..=20).map(|i| i as f32).collect();
        let a = atr(&highs, &lows, &closes, 14);
        for &v in &a[13..] {
            assert!(v > 0.0, "ATR should be positive, got {v}");
        }
    }

    #[test]
    fn vwap_with_constant_price_matches_price() {
        let prices: Vec<f32> = vec![100.0; 10];
        let volumes: Vec<f32> = (1..=10).map(|i| i as f32).collect();
        let v = vwap(&prices, &volumes);
        for &p in &v {
            assert!((p - 100.0).abs() < 1e-3);
        }
    }

    #[test]
    fn vwap_handles_length_mismatch() {
        let v = vwap(&[1.0, 2.0], &[1.0]);
        assert!(v.iter().all(|x| x.is_nan()));
    }

    #[test]
    fn ichimoku_components_populated() {
        let highs: Vec<f32> = (1..=80).map(|i| i as f32 + 1.0).collect();
        let lows: Vec<f32> = (1..=80).map(|i| i as f32 - 1.0).collect();
        let closes: Vec<f32> = (1..=80).map(|i| i as f32).collect();
        let ich = ichimoku(&highs, &lows, &closes, 9, 26, 52);
        // Tenkan populated from index 8 (window 9)
        assert!(ich.tenkan[8].is_finite());
        // Kijun populated from index 25
        assert!(ich.kijun[25].is_finite());
        // Senkou B populated from index 51, then shifted
        assert!(ich.senkou_b[51 + 26].is_finite());
        // Chikou shifted back
        assert!(ich.chikou[0].is_finite());
    }

    #[test]
    fn rsi_bounded_zero_to_hundred() {
        let v: Vec<f32> = (1..=30).map(|i| i as f32).collect(); // all gains
        let r = rsi(&v, 14);
        for &x in &r[14..] {
            assert!(x >= 0.0 && x <= 100.0);
        }
        // monotonic up should yield RSI near 100
        assert!(r[29] > 99.0);
    }
}
