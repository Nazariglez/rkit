use crate::ecs::app::App;
use crate::ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;

pub trait Screen:
    Resource + std::fmt::Debug + std::hash::Hash + Clone + Eq + Send + 'static
{
    fn add_schedules(app: &mut App) -> &mut App {
        app
    }
}

#[derive(Event, Debug, Clone, Copy)]
pub(crate) struct ChangeScreenEvt<S: Screen>(pub S);

#[derive(ScheduleLabel, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ScreenSchedule<SL: ScheduleLabel, S: Screen>(pub SL, pub S);

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
        world.send_event(ChangeScreenEvt(self.0.clone()));
    }
}

pub(crate) fn change_screen_event_system<S: Screen>(world: &mut World) {
    world.resource_scope(|world, evt: Mut<Events<ChangeScreenEvt<S>>>| {
        let mut cursor = evt.get_cursor();
        for evt in cursor.read(&evt) {
            let screen = evt.0.clone();
            if let Some(last_screen) = world.remove_resource::<S>() {
                log::debug!("Screen: OnExit({:?})", last_screen);
                world.run_schedule(OnExit(last_screen.clone()));
                log::debug!(
                    "Screen: OnChange(from: {:?}, to: {:?})",
                    last_screen,
                    screen
                );
                world.run_schedule(OnChange {
                    from: last_screen,
                    to: screen.clone(),
                });
            }

            world.insert_resource(screen.clone());
            log::debug!("Screen: OnEnter({:?})", screen);
            world.run_schedule(OnEnter(screen));
        }
    });
}
