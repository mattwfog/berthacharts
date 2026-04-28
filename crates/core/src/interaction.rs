//! Declarative interaction metadata attached to a scene.
//!
//! These primitives describe browser interaction affordances without coupling
//! framework bindings to renderer geometry. For example, an annotation overlay
//! can snap to authored data anchors even when the rendered mark is a ribbon,
//! rectangle, or future non-WebGL backend primitive.

/// Semantic category for a snap anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SnapKind {
    /// A data point or glyph center.
    Point,
    /// A node or region center.
    Node,
    /// A line, edge, or flow midpoint.
    Edge,
    /// A rectangular or bounded element center.
    Center,
    /// A guide, threshold, or reference anchor.
    Guide,
}

/// A screen-local point that annotation and interaction layers may snap to.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapTarget {
    /// Anchor x in chart-local CSS pixels.
    pub x: f32,
    /// Anchor y in chart-local CSS pixels.
    pub y: f32,
    /// Visual or hit radius in CSS pixels.
    pub radius: f32,
    /// Semantic target category.
    pub kind: SnapKind,
    /// Optional accessible/debug label for the anchor.
    pub label: Option<String>,
    /// Higher priority wins when multiple targets are close to the cursor.
    pub priority: i16,
}

impl SnapTarget {
    /// Build a snap target at `(x, y)` with a semantic kind.
    #[must_use]
    pub const fn new(x: f32, y: f32, kind: SnapKind) -> Self {
        Self {
            x,
            y,
            radius: 6.0,
            kind,
            label: None,
            priority: 0,
        }
    }

    /// Set the snap radius in CSS pixels.
    #[must_use]
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Attach a human-readable label to this target.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set priority for resolving nearby targets.
    #[must_use]
    pub const fn with_priority(mut self, priority: i16) -> Self {
        self.priority = priority;
        self
    }
}

/// A named set of snap targets.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapTargetSet {
    /// Optional set name, useful for diagnostics and UI toggles.
    pub name: Option<String>,
    /// Snap anchors in author order.
    pub targets: Vec<SnapTarget>,
    /// Whether this set is active by default.
    pub enabled: bool,
}

impl SnapTargetSet {
    /// Build an enabled snap target set.
    #[must_use]
    pub fn new(targets: Vec<SnapTarget>) -> Self {
        Self {
            name: None,
            targets,
            enabled: true,
        }
    }

    /// Set a human-readable target set name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set whether this target set is active by default.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Interaction metadata known to core.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Interaction {
    /// Annotation and pointer snap anchors.
    SnapTargets(SnapTargetSet),
}

#[cfg(test)]
mod tests {
    use super::{SnapKind, SnapTarget, SnapTargetSet};

    #[test]
    fn snap_target_builders_preserve_author_metadata() {
        let target = SnapTarget::new(12.0, 24.0, SnapKind::Guide)
            .with_radius(9.0)
            .with_label("target")
            .with_priority(4);

        assert_eq!(target.x, 12.0);
        assert_eq!(target.y, 24.0);
        assert_eq!(target.radius, 9.0);
        assert_eq!(target.kind, SnapKind::Guide);
        assert_eq!(target.label.as_deref(), Some("target"));
        assert_eq!(target.priority, 4);
    }

    #[test]
    fn snap_target_set_defaults_to_enabled() {
        let set = SnapTargetSet::new(vec![SnapTarget::new(1.0, 2.0, SnapKind::Point)])
            .with_name("points");

        assert!(set.enabled);
        assert_eq!(set.name.as_deref(), Some("points"));
        assert_eq!(set.targets.len(), 1);
    }
}
