//! Chart: a view into a [`Workspace`] with its own scene and transform DAG.
//!
//! A `Chart` owns:
//!
//! - A reference to the shared [`Workspace`].
//! - The current declarative [`Scene`].
//! - The transform [`Dag`] producing datasets consumed by marks.
//! - A [`Picker`] built from current mark bounds.
//! - A dirty flag for driving external rAF loops.
//!
//! Bindings (React, Leptos) wrap a `Chart` and push state into it; they do
//! not subclass it or mutate its internals.

use std::sync::Arc;

use crate::dag::Dag;
use crate::event::{ChartEvent, Subscription};
use crate::interaction::{Interaction, SnapTarget};
use crate::mark::{PickCtx, PickHit};
use crate::picker::Picker;
use crate::scene::{Scene, Viewport};
use crate::selection::Selection;
use crate::workspace::Workspace;

/// Tracked state after the last prepare pass.
#[derive(Debug, Default, Clone, Copy)]
struct DirtyFlags {
    /// True when the scene graph changed.
    scene: bool,
    /// True when the viewport changed.
    viewport: bool,
    /// True when transform DAG outputs may have changed.
    data: bool,
}

impl DirtyFlags {
    fn any(self) -> bool {
        self.scene || self.viewport || self.data
    }
    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// A chart view bound to a workspace.
pub struct Chart {
    workspace: Arc<Workspace>,
    scene: Scene,
    dag: Dag,
    #[allow(dead_code)] // Retained for the prepared spatial index path.
    picker: Picker,
    dirty: DirtyFlags,
}

impl std::fmt::Debug for Chart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chart")
            .field("scene", &self.scene)
            .field("dag", &self.dag)
            .field("dirty", &self.dirty)
            .finish_non_exhaustive()
    }
}

impl Chart {
    /// Construct a chart bound to `workspace` with the given initial viewport.
    #[must_use]
    pub fn new(workspace: Arc<Workspace>, viewport: Viewport) -> Self {
        Self {
            workspace,
            scene: Scene::new(viewport),
            dag: Dag::new(),
            picker: Picker::new(),
            dirty: DirtyFlags {
                scene: true,
                viewport: true,
                data: true,
            },
        }
    }

    /// Access the shared workspace.
    #[must_use]
    pub fn workspace(&self) -> &Arc<Workspace> {
        &self.workspace
    }

    /// Replace the scene wholesale. Bindings call this every time the
    /// declarative description changes (analogous to React's render).
    pub fn set_scene(&mut self, scene: Scene) {
        self.scene = scene;
        self.dirty.scene = true;
        self.workspace.emit(&ChartEvent::DirtyRequested);
    }

    /// Replace the viewport (size / DPR / plot area).
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.scene.viewport = viewport;
        self.dirty.viewport = true;
        self.workspace.emit(&ChartEvent::ViewportChanged);
        self.workspace.emit(&ChartEvent::DirtyRequested);
    }

    /// Borrow the current scene.
    #[must_use]
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Collect active scene-level snap anchors in author order.
    #[must_use]
    pub fn snap_targets(&self) -> Vec<SnapTarget> {
        self.scene
            .interactions
            .iter()
            .flat_map(|interaction| match interaction {
                Interaction::SnapTargets(set) if set.enabled => set.targets.clone(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// Borrow the transform DAG for inspection / mutation.
    #[must_use]
    pub fn dag(&self) -> &Dag {
        &self.dag
    }

    /// Mutable access to the DAG — bindings add / remove transform nodes here.
    pub fn dag_mut(&mut self) -> &mut Dag {
        self.dirty.data = true;
        &mut self.dag
    }

    /// True when the chart has pending state the renderer hasn't seen.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty.any()
    }

    /// Manually mark the chart dirty (e.g. after external changes the core
    /// can't observe). Emits [`ChartEvent::DirtyRequested`].
    pub fn request_redraw(&mut self) {
        self.dirty.data = true;
        self.workspace.emit(&ChartEvent::DirtyRequested);
    }

    /// Acknowledge that a frame has been rendered — clears dirty flags and
    /// emits [`ChartEvent::Rendered`]. Renderers call this at the end of a
    /// successful draw.
    pub fn mark_rendered(&mut self) {
        self.dirty.clear();
        self.workspace.emit(&ChartEvent::Rendered);
    }

    /// Snapshot the current selection.
    #[must_use]
    pub fn selection(&self) -> Selection {
        self.workspace.selection()
    }

    /// Hit-test a screen-local point against the picker.
    #[must_use]
    pub fn pick(&self, point: (f32, f32)) -> Option<PickHit> {
        let scales = self.workspace.scales();
        let datasets = self.workspace.datasets();
        let selection = self.workspace.selection();

        for layer in self.scene.layers.iter().rev() {
            let Some(coord) = self.workspace.coord(layer.coord) else {
                continue;
            };
            let ctx = PickCtx::new(
                coord.as_ref(),
                &scales,
                &datasets,
                &selection,
                self.scene.viewport.plot_area,
                self.scene.viewport.device_pixel_ratio,
            );
            for mark in layer.marks.iter().rev() {
                if let Some(hit) = mark.pick(&ctx, point) {
                    return Some(hit);
                }
            }
        }

        None
    }

    /// Subscribe to events on the underlying workspace. Convenience wrapper
    /// around [`Workspace::subscribe`].
    pub fn subscribe<F>(&self, f: F) -> Subscription
    where
        F: Fn(&ChartEvent) + Send + Sync + 'static,
    {
        self.workspace.subscribe(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Chart;
    use crate::{Interaction, Scene, SnapKind, SnapTarget, SnapTargetSet, Viewport, Workspace};

    #[test]
    fn snap_targets_collect_enabled_scene_interactions() {
        let workspace = Workspace::new();
        let viewport = Viewport::full(320, 240, 1.0);
        let mut scene = Scene::new(viewport);
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(vec![SnapTarget::new(10.0, 20.0, SnapKind::Point)])
                .with_name("enabled"),
        ));
        scene.interactions.push(Interaction::SnapTargets(
            SnapTargetSet::new(vec![SnapTarget::new(30.0, 40.0, SnapKind::Guide)])
                .with_name("disabled")
                .with_enabled(false),
        ));

        let mut chart = Chart::new(workspace, viewport);
        chart.set_scene(scene);

        let targets = chart.snap_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].x, 10.0);
        assert_eq!(targets[0].kind, SnapKind::Point);
    }
}
