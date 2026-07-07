//! # `berthacharts-finance`
//!
//! Finance-domain marks and indicators: candlestick + OHLC bars, moving
//! averages (SMA/EMA), Bollinger bands, RSI. Built on the same core traits
//! as the rest of Bertha — every finance chart is a [`berthacharts_core::ChartSpec`].
//!
//! ## Quick start
//!
//! ```
//! use berthacharts_finance::{Candle, CandlestickSpec, Overlay};
//! use berthacharts_finance::core::{ChartSize, Workspace};
//!
//! let candles = vec![
//!     Candle::new(0, 100.0, 105.0, 99.0, 102.0),
//!     Candle::new(1, 102.0, 107.0, 101.0, 106.0),
//! ];
//! let chart = CandlestickSpec::new(candles)
//!     .with_overlay(Overlay::Sma { window: 5, color: [1.0, 1.0, 0.0, 0.8] })
//!     .with_overlay(Overlay::Bollinger { window: 20, k: 2.0, color: [0.6, 0.6, 0.9, 0.6] })
//!     .try_build_chart(Workspace::new(), ChartSize::new(800, 400))?;
//! assert_eq!(chart.scene().layers.len(), 1);
//! # Ok::<(), berthacharts_finance::CandlestickError>(())
//! ```

#![forbid(unsafe_code)]

pub mod candlestick;
pub mod indicators;

pub use berthacharts_core as core;
pub use candlestick::{
    Candle, CandleBar, CandleStyle, CandlestickError, CandlestickLayout, CandlestickOptions,
    CandlestickSpec, Overlay, OverlayLine,
};
pub use indicators::{
    atr, bollinger_bands, exponential_moving_average, ichimoku, macd, moving_average, obv, rsi,
    stochastic, vwap, williams_r, BollingerBands, Ichimoku, Macd, Stochastic,
};
