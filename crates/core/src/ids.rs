//! Stable identifiers for scene entities.
//!
//! All IDs are cheap `Copy` newtypes. Users mint them externally (stable across
//! frames, same rule as React keys) so the scene diff can match old and new
//! nodes by identity rather than structural equality.

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $inner:ty) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            #[doc = "Construct an ID from its raw representation."]
            #[must_use]
            pub const fn new(raw: $inner) -> Self { Self(raw) }

            #[doc = "Access the raw integer value."]
            #[must_use]
            pub const fn get(self) -> $inner { self.0 }
        }

        impl From<$inner> for $name {
            fn from(raw: $inner) -> Self { Self(raw) }
        }
    };
}

define_id!(
    /// Identifier for a [`crate::Scale`] registered on a [`crate::Workspace`].
    ScaleId, u32
);
define_id!(
    /// Identifier for a [`crate::Dataset`] registered on a [`crate::Workspace`].
    DatasetId, u32
);
define_id!(
    /// Identifier for a [`crate::Transform`] node in the DAG.
    TransformId, u32
);
define_id!(
    /// Identifier for a [`crate::Layer`] in a [`crate::Scene`].
    LayerId, u32
);
define_id!(
    /// Stable identifier for a [`crate::Mark`] — required for diffing across frames.
    MarkId, u64
);
define_id!(
    /// Identifier for a named [`crate::Selection`] on a [`crate::Workspace`].
    SelectionId, u32
);
