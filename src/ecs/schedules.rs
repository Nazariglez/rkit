pub use bevy_ecs::schedule::{ExecutorKind, ScheduleLabel};

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnSetup;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnginePreFrame;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnginePostFrame;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPreFrame;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPostFrame;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPreUpdate;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnUpdate;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPostUpdate;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPreFixedUpdate(pub u8);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnFixedUpdate(pub u8);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPostFixedUpdate(pub u8);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPreRender;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnRender;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnPostRender;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnAudio;

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnCleanup;
