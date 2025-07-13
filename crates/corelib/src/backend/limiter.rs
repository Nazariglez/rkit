use std::time::{Duration, Instant};

use crate::time::fps;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LimitMode {
    Auto,
    Manual(Duration),
    #[default]
    Off,
}

impl LimitMode {
    #[inline]
    pub fn is_enabled(&self) -> bool {
        !matches!(self, LimitMode::Off)
    }

    pub fn from_fps(fps: f64) -> Self {
        LimitMode::Manual(Duration::from_secs_f64(1.0 / fps))
    }
}

pub(super) struct FpsLimiter {
    mode: LimitMode,
    target_delta: Duration,
    last_frame_end: Instant,
    oversleep: Duration,
}

impl FpsLimiter {
    #[inline]
    pub fn new(mode: LimitMode, monitor_hz: Option<f64>) -> Self {
        FpsLimiter {
            mode,
            target_delta: duration_from_mode(mode, monitor_hz),
            last_frame_end: Instant::now(),
            oversleep: Duration::ZERO,
        }
    }

    #[inline(always)]
    pub fn tick(&mut self) {
        let is_enabled = self.mode.is_enabled();
        let has_dt = self.target_delta > Duration::ZERO;

        let prev_elapsed = self.last_frame_end.elapsed();

        let can_tick = is_enabled && has_dt;
        if can_tick {
            let sleep_time = self
                .target_delta
                .saturating_sub(prev_elapsed + self.oversleep);
            let can_sleep = sleep_time > Duration::ZERO;
            println!(
                "prev_elapsed {prev_elapsed:?} | target_delta {:?} | can_sleep = {can_sleep:?} | oversleep = {:?}",
                self.target_delta, self.oversleep,
            );

            if can_sleep {
                println!("sleep {sleep_time:?}");
                spin_sleep::sleep(sleep_time);
            }
        }

        let after_elapsed = self.last_frame_end.elapsed();

        self.last_frame_end = Instant::now();
        self.oversleep = after_elapsed.saturating_sub(self.target_delta);

        println!(
            "post frame {after_elapsed:?}, oversleep {:?} -> final fps {}",
            self.oversleep,
            fps()
        );
    }

    #[inline(always)]
    pub fn update(&mut self, monitor_hz: Option<f64>) {
        self.update_mode(self.mode, monitor_hz);
    }

    #[inline(always)]
    pub fn update_mode(&mut self, mode: LimitMode, monitor_hz: Option<f64>) {
        self.mode = mode;
        self.target_delta = duration_from_mode(mode, monitor_hz);
    }
}

#[inline(always)]
fn duration_from_mode(mode: LimitMode, hz: Option<f64>) -> Duration {
    let auto_dt = Duration::from_secs_f64(1.0 / hz.unwrap_or(60.0));
    match mode {
        LimitMode::Auto => auto_dt,
        LimitMode::Manual(dt) => dt.max(auto_dt),
        LimitMode::Off => Duration::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const EPS: f64 = 1e-9;

    #[test]
    fn limit_mode_is_enabled() {
        assert!(LimitMode::Auto.is_enabled());
        assert!(LimitMode::Manual(Duration::from_secs(1)).is_enabled());
        assert!(!LimitMode::Off.is_enabled());
    }

    #[test]
    fn from_framerate_computes_inverse() {
        let fps = 30.0;
        let dt = LimitMode::from_fps(fps);
        assert!(matches!(dt, LimitMode::Manual(_)));

        if let LimitMode::Manual(d) = dt {
            let expected = 1.0 / fps;
            let actual = d.as_secs_f64();
            assert!(
                (actual - expected).abs() < EPS,
                "got {actual}, expected {expected}"
            );
        }
    }

    #[test]
    fn duration_from_mode_auto_default() {
        let auto = duration_from_mode(LimitMode::Auto, None);
        assert!((auto.as_secs_f64() - 1.0 / 60.0).abs() < EPS);
    }

    #[test]
    fn duration_from_mode_manual_minimum() {
        let small = Duration::from_secs_f64(1.0 / 120.0);
        let dt = duration_from_mode(LimitMode::Manual(small), Some(60.0));
        assert_eq!(dt, small);

        let large = Duration::from_secs_f64(1.0 / 30.0);
        let dt2 = duration_from_mode(LimitMode::Manual(large), Some(60.0));
        assert_eq!(dt2, Duration::from_secs_f64(1.0 / 60.0));
    }

    #[test]
    fn duration_from_mode_off_is_zero() {
        let off = duration_from_mode(LimitMode::Off, Some(144.0));
        assert_eq!(off, Duration::ZERO);
    }

    #[test]
    fn fps_limiter_new_and_update_sets_target_dt() {
        let mut limiter = FpsLimiter::new(LimitMode::Auto, Some(144.0));
        let expected = Duration::from_secs_f64(1.0 / 144.0);
        assert_eq!(limiter.target_delta, expected);

        let manual_large = Duration::from_secs_f64(1.0 / 30.0);
        limiter.update_mode(LimitMode::Manual(manual_large), Some(60.0));
        let expected2 = Duration::from_secs_f64(1.0 / 60.0);
        assert_eq!(limiter.target_delta, expected2);

        let manual_small = Duration::from_secs_f64(1.0 / 120.0);
        limiter.update_mode(LimitMode::Manual(manual_small), Some(60.0));
        assert_eq!(limiter.target_delta, manual_small);

        limiter.update_mode(LimitMode::Off, None);
        assert_eq!(limiter.target_delta, Duration::ZERO);
        assert!(!limiter.mode.is_enabled());
    }

    #[test]
    fn tick_does_not_panic_when_off() {
        let mut limiter = FpsLimiter::new(LimitMode::Off, None);
        limiter.tick();
    }
}
