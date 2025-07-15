use std::time::Duration;
use spin_sleep_util::Interval;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LimitMode {
    Auto,
    Target(Duration),
    #[default]
    Disabled,
}

impl LimitMode {
    #[inline]
    pub fn from_fps(fps: f64) -> Self {
        LimitMode::Target(Duration::from_secs_f64(1.0 / fps))
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        !matches!(self, LimitMode::Disabled)
    }
}

pub(super) struct FpsLimiter {
    mode: LimitMode,
    interval: Interval,
}

impl FpsLimiter {
    #[inline]
    pub fn new(mode: LimitMode, monitor_hz: Option<f64>) -> Self {
        let dt = match mode {
            LimitMode::Disabled => Duration::from_secs_f64(1.0 / 60.0),
            _ => duration_from_mode(mode, monitor_hz),
        };

        if mode.is_enabled() {
            log::debug!("FPSLimiter enabled with mode={mode:?}");
        }

        FpsLimiter {
            mode,
            interval: spin_sleep_util::interval(dt),
        }
    }

    #[inline(always)]
    pub fn tick(&mut self) {
        let is_enabled = self.mode.is_enabled();
        if !is_enabled {
            return;
        }

        self.interval.tick();
    }

    #[inline(always)]
    pub fn update(&mut self, monitor_hz: Option<f64>) {
        self.update_mode(self.mode, monitor_hz);
    }

    #[inline(always)]
    pub fn update_mode(&mut self, mode: LimitMode, monitor_hz: Option<f64>) {
        if self.mode != mode {
            log::debug!("FPSLimiter mode changed to {mode:?}");
            self.mode = mode;
        }

        let is_enabled = self.mode.is_enabled();
        if !is_enabled {
            return;
        }

        let dt = duration_from_mode(self.mode, monitor_hz);
        if dt <= Duration::ZERO {
            return;
        }

        let needs_update = dt != self.interval.period();
        if !needs_update {
            return;
        }

        self.interval.set_period(dt);
        log::debug!("FPSLimiter delta set to {dt:?}");
    }
}

#[inline(always)]
fn duration_from_mode(mode: LimitMode, hz: Option<f64>) -> Duration {
    let auto_dt = Duration::from_secs_f64(1.0 / hz.unwrap_or(60.0));
    match mode {
        LimitMode::Auto => auto_dt,
        LimitMode::Target(dt) => dt.max(auto_dt),
        LimitMode::Disabled => Duration::ZERO,
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
        assert!(LimitMode::Target(Duration::from_secs(1)).is_enabled());
        assert!(!LimitMode::Disabled.is_enabled());
    }

    #[test]
    fn from_framerate_computes_inverse() {
        let fps = 30.0;
        let dt = LimitMode::from_fps(fps);
        assert!(matches!(dt, LimitMode::Target(_)));

        if let LimitMode::Target(d) = dt {
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
        let f120 = Duration::from_secs_f64(1.0 / 120.0);
        let f60 = Duration::from_secs_f64(1.0 / 60.0);
        let dt = duration_from_mode(LimitMode::Target(f120), Some(60.0));
        assert_eq!(dt, f60);

        let f30 = Duration::from_secs_f64(1.0 / 30.0);
        let dt2 = duration_from_mode(LimitMode::Target(f30), Some(60.0));
        assert_eq!(dt2, f30);
    }

    #[test]
    fn duration_from_mode_off_is_zero() {
        let off = duration_from_mode(LimitMode::Disabled, Some(144.0));
        assert_eq!(off, Duration::ZERO);
    }

    #[test]
    fn tick_does_not_panic_when_off() {
        let mut limiter = FpsLimiter::new(LimitMode::Disabled, None);
        limiter.tick();
    }
}
