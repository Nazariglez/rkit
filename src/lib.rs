pub mod tween;
pub mod utils;

// #[cfg(feature = "save")]
pub mod save;

#[cfg(feature = "postfx")]
pub mod postfx;

#[cfg(feature = "random")]
pub mod random;

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "ecs")]
pub mod ecs;

pub mod or_panic;
pub mod particles;
pub mod prelude;
pub mod timer;

#[cfg(feature = "egui")]
pub mod egui;

#[doc(inline)]
pub use corelib::*;

#[doc(inline)]
#[cfg(feature = "draw")]
pub use draw;

#[doc(inline)]
#[cfg(feature = "audio")]
pub use audio;

#[doc(inline)]
#[cfg(feature = "assets")]
pub use assets;

pub use macros;
