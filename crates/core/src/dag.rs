//! Transform DAG: lazy, memoized dataflow graph.
//!
//! Nodes are [`crate::Transform`] instances. Edges carry dataset outputs.
//! Results are cached by `(node_fingerprint, input_fingerprints, selection_fp)`;
//! a run only re-executes nodes whose key has changed.
//!
//! v0.1 ships the type surface — the execution algorithm lands in a follow-up.

use std::sync::Arc;

use ahash::AHashMap;
use smallvec::SmallVec;

use crate::dataset::{Dataset, DatasetId};
use crate::ids::TransformId;
use crate::selection::Selection;
use crate::transform::Transform;

/// 64-bit cache key used throughout the DAG.
pub type Fingerprint = u64;

/// Identifier for a node in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Construct from raw.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Raw representation.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// An input edge into a DAG node.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NodeInput {
    /// A raw dataset from the workspace registry.
    Dataset(DatasetId),
    /// Output of another DAG node.
    Node(NodeId),
}

/// A single DAG node wrapping a transform with its inputs.
pub struct Node {
    /// Stable id.
    pub id: NodeId,
    /// User-assigned logical id (exposed so bindings can reference it).
    pub transform_id: TransformId,
    /// The transform.
    pub transform: Arc<dyn Transform>,
    /// Input edges.
    pub inputs: SmallVec<[NodeInput; 2]>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("transform_id", &self.transform_id)
            .field("transform", &self.transform.name())
            .field("inputs", &self.inputs)
            .finish()
    }
}

/// Cached output entry.
///
/// Fields are reserved for the execution algorithm that lands in v0.1.1.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CacheEntry {
    /// Joint cache key.
    key: Fingerprint,
    /// Memoized output.
    output: Arc<Dataset>,
}

/// The transform DAG.
#[derive(Debug, Default)]
pub struct Dag {
    nodes: AHashMap<NodeId, Node>,
    cache: AHashMap<NodeId, CacheEntry>,
    next_node: u32,
}

impl Dag {
    /// Empty DAG.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node. Returns its allocated [`NodeId`].
    pub fn insert(
        &mut self,
        transform_id: TransformId,
        transform: Arc<dyn Transform>,
        inputs: SmallVec<[NodeInput; 2]>,
    ) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node = self.next_node.wrapping_add(1);
        self.nodes.insert(
            id,
            Node {
                id,
                transform_id,
                transform,
                inputs,
            },
        );
        id
    }

    /// Remove a node and invalidate its cache entry.
    pub fn remove(&mut self, id: NodeId) -> Option<Node> {
        self.cache.remove(&id);
        self.nodes.remove(&id)
    }

    /// Borrow a node.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Iterate nodes.
    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &Node)> {
        self.nodes.iter()
    }

    /// Clear all cached outputs — forces the next run to recompute everything.
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    /// Joint fingerprint of the entire DAG structure plus selection state.
    #[must_use]
    pub fn fingerprint(&self, selection: &Selection) -> Fingerprint {
        let mut pairs: Vec<_> = self
            .nodes
            .iter()
            .map(|(k, n)| (*k, n.transform.fingerprint()))
            .collect();
        pairs.sort_unstable_by_key(|(k, _)| *k);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (k, fp) in pairs {
            h ^= u64::from(k.get());
            h = h.wrapping_mul(0x0100_0000_01b3);
            h ^= fp;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h ^ selection.fingerprint()
    }
}
