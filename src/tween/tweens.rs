use super::{EaseFn, Interpolable, LINEAR};

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum ApplyState {
    #[default]
    Idle,
    Applied,
    Ended,
}

impl ApplyState {
    pub fn is_done(self) -> bool {
        matches!(self, Self::Ended)
    }
}

#[derive(Clone, Copy)]
enum RepeatMode {
    Never,
    Forever,
    Times(u32),
}

#[derive(Clone, Copy)]
enum State {
    Idle,
    Started,
    Ended,
}

#[derive(Clone, Copy)]
pub struct Tween<T: Interpolable> {
    from: T,
    to: T,
    repeat_mode: RepeatMode,
    state: State,
    easing: EaseFn,
    time: f32,
    delay: f32,
    yoyo_enabled: bool,
    yoyo_back: bool,
    elapsed_time: f32,
    elapsed_delay: f32,
    repeated: u32,
    value: T,
    apply_end: bool,
}

impl<T: Interpolable> Tween<T> {
    pub fn new(from: T, to: T, time: f32) -> Self {
        Self {
            from,
            to,
            time,
            state: State::Idle,
            easing: LINEAR,
            delay: 0.0,
            repeat_mode: RepeatMode::Never,
            yoyo_enabled: false,
            yoyo_back: false,
            elapsed_time: 0.0,
            elapsed_delay: 0.0,
            repeated: 0,
            value: from,
            apply_end: false,
        }
    }

    pub fn tick(&mut self, delta: f32) {
        if !can_update(self) {
            return;
        }

        if self.elapsed_delay < self.delay {
            self.elapsed_delay += delta;
            return;
        }

        let time = if self.yoyo_enabled {
            self.time * 0.5
        } else {
            self.time
        };

        if self.elapsed_time < time {
            let current_time = self.elapsed_time + delta;
            let did_finish = current_time >= time;

            self.elapsed_time = if did_finish { time } else { current_time };

            self.value = self
                .from
                .interpolate(self.to, self.elapsed_time / time, self.easing);

            if did_finish {
                if self.yoyo_enabled && !self.yoyo_back {
                    self.yoyo_back = true;
                    std::mem::swap(&mut self.from, &mut self.to);
                    self.elapsed_time = 0.0;
                    return;
                }

                let repeat = match self.repeat_mode {
                    RepeatMode::Forever => true,
                    RepeatMode::Times(times) => self.repeated < times,
                    _ => false,
                };

                if repeat {
                    self.repeated += 1;
                    self.elapsed_time = 0.0;

                    if self.yoyo_enabled && self.yoyo_back {
                        self.yoyo_back = false;
                        std::mem::swap(&mut self.from, &mut self.to);
                    }

                    return;
                }

                self.state = State::Ended;
            }
        }
    }

    pub fn start(&mut self) {
        match self.state {
            State::Idle => self.state = State::Started,
            State::Ended => {
                self.reset();
                self.state = State::Started;
            }
            _ => {}
        }
    }

    pub fn stop(&mut self) {
        if matches!(self.state, State::Started) {
            self.state = State::Idle;
        }
    }

    pub fn reset(&mut self) {
        self.state = State::Idle;
        self.elapsed_time = 0.0;
        self.elapsed_delay = 0.0;
        self.repeated = 0;
        self.value = self.from;
    }

    pub fn set_repeat(&mut self, times: u32) {
        self.repeat_mode = RepeatMode::Times(times);
    }

    pub fn set_repeat_forever(&mut self, repeat: bool) {
        self.repeat_mode = if repeat {
            RepeatMode::Forever
        } else {
            RepeatMode::Never
        };
    }

    pub fn set_yoyo(&mut self, yoyo: bool) {
        self.yoyo_enabled = yoyo;
    }

    pub fn set_easing(&mut self, easing: EaseFn) {
        self.easing = easing;
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn running_time(&self) -> f32 {
        self.time * (self.repeated as f32) + self.elapsed_time
    }

    pub fn is_started(&self) -> bool {
        matches!(self.state, State::Started)
    }

    pub fn is_ended(&self) -> bool {
        matches!(self.state, State::Ended)
    }

    pub fn apply<F: FnOnce(T)>(&mut self, cb: F) -> ApplyState {
        let can_apply = self.is_started() || self.is_ended();
        if !can_apply {
            return ApplyState::Idle;
        }

        cb(self.value);

        // avoid apply this after the end
        self.apply_end = self.is_ended();
        if self.apply_end {
            ApplyState::Ended
        } else {
            ApplyState::Applied
        }
    }
}

#[inline]
fn can_update<T: Interpolable>(tween: &Tween<T>) -> bool {
    matches!(tween.state, State::Started) && tween.time > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let tween = Tween::new(0.0, 100.0, 1.0);
        assert_eq!(tween.value(), 0.0);
        assert!(matches!(tween.state, State::Idle));
    }

    #[test]
    fn test_start_and_stop() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        assert!(!tween.is_started());
        tween.start();
        assert!(tween.is_started());
        tween.stop();
        assert!(!tween.is_started());
    }

    #[test]
    fn test_tick_updates_value() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.start();
        tween.tick(0.5);
        assert_eq!(tween.value(), 50.0);
        tween.tick(0.5);
        assert_eq!(tween.value(), 100.0);
        assert!(tween.is_ended());
    }

    #[test]
    fn test_repeat_mode_times() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_repeat(2);
        tween.start();

        tween.tick(1.0);
        assert_eq!(tween.value(), 100.0);
        assert!(!tween.is_ended());

        tween.tick(1.0);
        assert_eq!(tween.value(), 100.0);
        assert!(!tween.is_ended());

        tween.tick(1.0);
        assert!(tween.is_ended());
    }

    #[test]
    fn test_repeat_forever() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_repeat_forever(true);
        tween.start();

        for _ in 0..10 {
            tween.tick(1.0);
            assert_eq!(tween.value(), 100.0);
            assert!(!tween.is_ended());
        }
    }

    #[test]
    fn test_yoyo_mode() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_yoyo(true);
        tween.start();

        tween.tick(1.0);
        assert_eq!(tween.value(), 100.0);
        assert!(!tween.is_ended());

        tween.tick(1.0); // Yoyo back
        assert_eq!(tween.value(), 0.0);
        assert!(tween.is_ended());
    }

    #[test]
    fn test_reset() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.start();
        tween.tick(0.5);
        tween.reset();

        assert!(!tween.is_started());
        assert_eq!(tween.elapsed_time, 0.0);
        assert_eq!(tween.elapsed_delay, 0.0);
        assert_eq!(tween.repeated, 0);
        assert_eq!(tween.value(), 0.0);
    }

    #[test]
    fn test_running_time() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_repeat(2);
        tween.start();

        tween.tick(1.0);
        assert_eq!(tween.running_time(), 1.0);
        tween.tick(1.0);
        assert_eq!(tween.running_time(), 2.0);
        tween.tick(1.0);
        assert_eq!(tween.running_time(), 3.0);
    }

    #[test]
    fn test_apply_updates_object() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        let mut obj = Object { position: 0.0 };

        tween.start();
        tween.tick(0.5);

        tween.apply(|value| {
            obj.position = value;
        });

        assert_eq!(obj.position, 50.0);
    }

    struct Object {
        position: f32,
    }

    #[test]
    fn test_apply_does_not_update_when_not_started() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        let mut obj = Object { position: 0.0 };

        tween.apply(|value| {
            obj.position = value;
        });

        assert_eq!(obj.position, 0.0);
    }

    #[test]
    fn test_apply_updates_object_with_yoyo() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_yoyo(true);
        let mut obj = Object { position: 0.0 };

        tween.start();

        tween.tick(0.5);
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 100.0);

        tween.tick(0.5);
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 0.0);
    }

    #[test]
    fn test_apply_multiple_ticks() {
        let mut tween = Tween::new(0.0, 100.0, 1.0);
        tween.set_repeat(2);
        let mut obj = Object { position: 0.0 };

        tween.start();

        tween.tick(1.0);
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 100.0);

        tween.tick(1.0);
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 100.0);

        tween.tick(1.0);
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 100.0);

        tween.tick(1.0);
        assert!(tween.is_ended());
        tween.apply(|value| {
            obj.position = value;
        });
        assert_eq!(obj.position, 100.0);
    }
}
