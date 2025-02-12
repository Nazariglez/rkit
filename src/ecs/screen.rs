use crate::ecs::app::App;
use crate::ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

pub trait Screen:
    Resource + std::fmt::Debug + std::hash::Hash + Clone + Eq + Send + 'static
{
    fn add_schedules(app: App) -> App {
        app
    }
}

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnExit<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnEnter<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OnChange<S: Screen> {
    pub from: S,
    pub to: S,
}

pub fn in_screen<S: Screen>(screen: S) -> impl FnMut(Option<Res<S>>) -> bool + Clone {
    move |s: Option<Res<S>>| s.is_some_and(|current| *current == screen)
}

#[derive(Clone, Copy)]
pub struct ChangeScreen<S: Screen>(pub S);

impl<S: Screen> Command for ChangeScreen<S> {
    fn apply(self, world: &mut World) {
        if let Some(last_screen) = world.remove_resource::<S>() {
            log::debug!("Screen: OnExit({:?})", last_screen);
            world.run_schedule(OnExit(last_screen.clone()));
            log::debug!(
                "Screen: OnChange(from: {:?}, to: {:?})",
                last_screen,
                self.0.clone()
            );
            world.run_schedule(OnChange {
                from: last_screen,
                to: self.0.clone(),
            });
        }

        world.insert_resource(self.0.clone());
        log::debug!("Screen: OnEnter({:?})", self.0);
        world.run_schedule(OnEnter(self.0));
    }
}
