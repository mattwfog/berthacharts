//! Typed chart-spec contracts.
//!
//! A chart spec is the reusable layer above marks and guides. Specs own data
//! schema, validation, layout, guide policy, and interaction metadata, then
//! compile into a normal [`crate::Chart`] for renderers and bindings.

use std::sync::Arc;

use crate::{Chart, Rect, Viewport, Workspace};

/// Logical chart size supplied to a chart spec during build.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartSize {
    /// Canvas width in CSS pixels.
    pub width: u32,
    /// Canvas height in CSS pixels.
    pub height: u32,
    /// Device pixel ratio.
    pub device_pixel_ratio: f32,
}

impl ChartSize {
    /// Build a logical chart size.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            device_pixel_ratio: 1.0,
        }
    }

    /// Set device pixel ratio.
    #[must_use]
    pub const fn with_device_pixel_ratio(mut self, device_pixel_ratio: f32) -> Self {
        self.device_pixel_ratio = device_pixel_ratio;
        self
    }

    /// Convert this size to a full-canvas viewport.
    #[must_use]
    pub fn full_viewport(self) -> Viewport {
        Viewport::full(self.width, self.height, self.device_pixel_ratio)
    }

    /// Convert this size to a viewport with an explicit plot area.
    #[must_use]
    pub const fn viewport_with_plot_area(self, plot_area: Rect) -> Viewport {
        Viewport {
            width: self.width,
            height: self.height,
            device_pixel_ratio: self.device_pixel_ratio,
            plot_area,
        }
    }
}

/// A reusable chart specification that can compile data into a chart scene.
pub trait ChartSpec {
    /// Build/validation error.
    type Error;

    /// Compile this spec into a chart attached to `workspace`.
    fn build_chart(&self, workspace: Arc<Workspace>, size: ChartSize)
        -> Result<Chart, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::ChartSize;
    use crate::Rect;

    #[test]
    fn chart_size_builds_full_viewport() {
        let viewport = ChartSize::new(320, 240)
            .with_device_pixel_ratio(2.0)
            .full_viewport();

        assert_eq!(viewport.width, 320);
        assert_eq!(viewport.height, 240);
        assert_eq!(viewport.device_pixel_ratio, 2.0);
        assert_eq!(viewport.plot_area, Rect::new(0.0, 0.0, 320.0, 240.0));
    }

    #[test]
    fn chart_size_builds_custom_plot_viewport() {
        let viewport =
            ChartSize::new(320, 240).viewport_with_plot_area(Rect::new(20.0, 30.0, 260.0, 180.0));

        assert_eq!(viewport.plot_area, Rect::new(20.0, 30.0, 260.0, 180.0));
    }
}
