//! Compatibility import path for owned ECS UI scene authoring.

mod command;
mod constructors;
mod scene;

pub use self::{
    command::CommandUISceneExt,
    scene::{UIEntityScope, UIScene},
};

pub mod ui {
    pub use super::constructors::{
        column, container, image, node, rich_text, row, text, with_entities,
    };
}
pub use super::{
    diagnostics::{UIHierarchyError, UIRuntimeError, UISceneError},
    events::{
        UIClick, UIDragInput, UIPointerEnter, UIPointerLeave, UIPointerPosition, UIPointerPressed,
        UIPointerReleased, UIScrollInput,
    },
    measure::{UIAvailableSpace, UIMeasure, UIMeasureInput},
};
