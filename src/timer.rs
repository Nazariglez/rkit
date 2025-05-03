#[derive(Debug, Clone)]
pub struct Timer {
    from: f32,
    to: f32,
    repeat_limit: u32,
    infinite: bool,

    elapsed: f32,
    just_finished: bool,
    just_finished_repeat: bool,
    finished: bool,
    repeated: u32,
}

impl Timer {
    pub fn new(time: f32) -> Self {
        Self {
            from: 0.0,
            to: time,
            repeat_limit: 0,
            infinite: false,

            elapsed: 0.0,
            just_finished: false,
            just_finished_repeat: false,
            finished: false,
            repeated: 0,
        }
    }

    /// Sets the starting point of the timer
    #[inline]
    pub fn with_from(mut self, from: f32) -> Self {
        self.from = from;
        self
    }

    /// Sets the number of repeats
    #[inline]
    pub fn with_repeat(mut self, repeat: u32) -> Self {
        self.repeat_limit = repeat;
        self
    }

    /// Sets whether the timer should repeat infinitely
    #[inline]
    pub fn with_infinite(mut self, infinite: bool) -> Self {
        self.infinite = infinite;
        self
    }

    /// Makes the timer finish immediately on the first tick
    #[inline]
    pub fn with_inmediate(mut self) -> Self {
        self.to = self.from;
        self
    }

    /// Advances the timer by `delta` seconds
    pub fn tick(&mut self, delta: f32) {
        //  clear the "just" flags
        if self.finished {
            self.just_finished = false;
            self.just_finished_repeat = false;
            return;
        }

        self.elapsed += delta;

        // check if we've reached or exceeded the target time
        if self.elapsed >= self.to {
            // if repeating is enabled (repeat limit > 0 or infinite), handle repeat logic.
            if self.repeat_limit > 0 || self.infinite {
                self.just_finished_repeat = true;
                self.repeated += 1;

                // if infinite or we haven't reached the repeat limit, reset the timer for the next cycle.
                if self.infinite || self.repeated < self.repeat_limit {
                    // carry over any extra time past the target.
                    self.elapsed = self.from + (self.elapsed - self.to);
                    self.just_finished = false;
                } else {
                    // this was the last cycle.
                    self.finished = true;
                    self.just_finished = true;
                    self.elapsed = self.to;
                }
            } else {
                // timer is a one-shot: mark it as finished.
                self.finished = true;
                self.just_finished = true;
                self.elapsed = self.to;
            }
        } else {
            // timer is still in progress.
            self.just_finished = false;
            self.just_finished_repeat = false;
        }
    }

    #[inline]
    pub fn end(&mut self) {
        self.elapsed = self.to;
        self.tick(0.0);
    }

    #[inline]
    pub fn just_finished(&self) -> bool {
        self.just_finished
    }

    #[inline]
    pub fn just_finished_repeat(&self) -> bool {
        self.just_finished_repeat
    }

    #[inline]
    pub fn finished(&self) -> bool {
        self.finished
    }

    #[inline]
    pub fn repeated(&self) -> u32 {
        self.repeated
    }

    #[inline]
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    #[inline]
    pub fn remaining(&self) -> f32 {
        self.to - self.elapsed
    }

    #[inline]
    pub fn progress(&self) -> f32 {
        self.elapsed / self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_shot() {
        let mut timer = Timer::new(1.0);
        timer.tick(0.5);
        assert!(!timer.finished(), "Timer should not be finished after 0.5s");
        assert!(
            !timer.just_finished(),
            "just_finished should be false mid-cycle"
        );

        timer.tick(0.6);
        assert!(
            timer.finished(),
            "Timer should be finished after accumulating 1.1s"
        );
        assert!(
            timer.just_finished(),
            "just_finished should be true when finishing"
        );
        assert_eq!(
            timer.elapsed(),
            timer.to,
            "Elapsed time should be exactly the target time"
        );
    }

    #[test]
    fn test_repeat() {
        let mut timer = Timer::new(1.0).with_repeat(2);

        timer.tick(1.0);
        assert!(
            !timer.finished(),
            "Timer should not be finished after first cycle"
        );
        assert!(
            timer.just_finished_repeat(),
            "just_finished_repeat should be true at cycle finish"
        );
        assert_eq!(timer.repeated(), 1, "One cycle should have been completed");

        timer.tick(0.5);
        assert!(
            !timer.finished(),
            "Timer should not be finished in the middle of a cycle"
        );

        timer.tick(0.5);
        assert!(
            timer.finished(),
            "Timer should be finished after second cycle"
        );
        assert!(
            timer.just_finished(),
            "just_finished should be true on final completion"
        );
        assert_eq!(timer.repeated(), 2, "Two cycles should have been completed");
    }

    #[test]
    fn test_infinite() {
        let mut timer = Timer::new(1.0).with_infinite(true);
        for i in 0..10 {
            timer.tick(1.0);
            assert!(
                !timer.finished(),
                "Infinite timer should never be marked finished"
            );
            assert!(
                timer.just_finished_repeat(),
                "just_finished_repeat should be true at each cycle finish"
            );
            assert_eq!(
                timer.repeated(),
                i + 1,
                "Cycle count should increment each time"
            );
        }
    }

    #[test]
    fn test_with_from_repeat() {
        let mut timer = Timer::new(1.0).with_from(0.5).with_repeat(3);
        timer.tick(1.2);
        assert!(
            !timer.finished(),
            "Timer should not be finished after first cycle with overflow"
        );
        assert!(
            timer.just_finished_repeat(),
            "just_finished_repeat should be true at cycle finish"
        );
        assert_eq!(timer.repeated(), 1, "One cycle should have been completed");
        assert!(
            (timer.elapsed() - 0.7).abs() < 1e-6,
            "Elapsed time should be carried over correctly"
        );

        timer.tick(0.3);
        assert!(
            timer.just_finished_repeat(),
            "just_finished_repeat should be true at cycle finish"
        );
        assert_eq!(timer.repeated(), 2, "Two cycles should have been completed");
        assert!(
            (timer.elapsed() - 0.5).abs() < 1e-6,
            "Elapsed time should be reset to starting point"
        );

        timer.tick(0.6);
        assert!(
            timer.finished(),
            "Timer should be finished after final cycle"
        );
        assert!(
            timer.just_finished(),
            "just_finished should be true on final completion"
        );
        assert_eq!(
            timer.repeated(),
            3,
            "Three cycles should have been completed"
        );
        assert_eq!(
            timer.elapsed(),
            timer.to,
            "Elapsed time should be exactly the target time on final completion"
        );
    }

    #[test]
    fn test_tick_after_finished() {
        let mut timer = Timer::new(1.0);
        timer.tick(1.2);
        assert!(timer.finished(), "Timer should be finished");
        assert!(
            timer.just_finished(),
            "just_finished should be true on final completion"
        );

        timer.tick(0.5);
        assert!(
            !timer.just_finished(),
            "just_finished should be false after ticking a finished timer"
        );
        assert!(
            !timer.just_finished_repeat(),
            "just_finished_repeat should be false after ticking a finished timer"
        );
        assert!(timer.finished(), "Timer should remain finished");
    }
}
