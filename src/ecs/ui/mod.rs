//! ECS UI supports command-builder authoring and owned scene composition over one runtime.
//!
//! Use [`CommandSpawnUIExt::spawn_ui_node`](command::CommandSpawnUIExt::spawn_ui_node) when
//! constructing or mutating a tree directly through [`Commands`](bevy_ecs::prelude::Commands).
//! Use [`UIScene`] when reusable functions should return owned UI subtrees. Both styles can
//! populate the same [`UILayout`](layout::UILayout) and share hierarchy, layout, rendering,
//! and pointer behavior. Constructors stay namespaced under [`ui`].
//!
//! ```
//! use rkit::{
//!     ecs::ui::{CommandUISceneExt, UIScene, ui, widgets::UIContainer},
//!     prelude::*,
//! };
//!
//! #[derive(Component, Clone, Copy)]
//! struct MainLayout;
//!
//! #[derive(Component)]
//! struct StartButton;
//!
//! fn button(label: impl Into<String>) -> UIScene {
//!     ui::container(UIContainer::default())
//!         .style(|style| style.size(180.0, 48.0).align_items_center())
//!         .child(ui::text(label))
//!         .insert(StartButton)
//! }
//!
//! fn spawn_menu(mut commands: Commands) {
//!     commands.spawn_ui(MainLayout, ui::column().child(button("Start")));
//! }
//! ```
//!
//! Current command-builder authoring remains supported alongside scenes:
//!
//! ```
//! use rkit::{
//!     ecs::ui::{command::CommandSpawnUIExt, widgets::UIContainer},
//!     prelude::*,
//! };
//!
//! #[derive(Component, Clone, Copy)]
//! struct MainLayout;
//!
//! fn spawn_menu(mut commands: Commands) {
//!     commands.spawn_ui_node(MainLayout, UIContainer::default());
//! }
//! ```

pub mod command;
pub mod components;
pub mod ctx;
mod diagnostics;
mod events;
pub mod experimental;
pub mod layout;
mod measure;
pub mod plugin;
pub(super) mod prelude;
#[cfg(feature = "ecs-ui-rsx")]
#[doc(hidden)]
pub mod rsx_widgets;
mod spawn;
pub mod style;
pub mod widgets;

pub use self::{
    diagnostics::{UIHierarchyError, UIRuntimeError, UISceneError},
    events::{
        UIClick, UIDragInput, UIPointerEnter, UIPointerLeave, UIPointerPosition, UIPointerPressed,
        UIPointerReleased, UIScrollInput,
    },
    experimental::{CommandUISceneExt, UIScene},
    measure::{UIAvailableSpace, UIMeasure, UIMeasureInput},
};

pub mod ui {
    pub use super::experimental::ui::*;
}

#[cfg(feature = "ecs-ui-rsx")]
#[doc(hidden)]
#[macro_export]
macro_rules! __rkit_rsx {
    ($($tokens:tt)*) => {
        $crate::macros::__rkit_rsx!($crate; $($tokens)*)
    };
}

#[cfg(feature = "ecs-ui-rsx")]
pub use crate::{__rkit_rsx as rsx, macros::ui_widget};
