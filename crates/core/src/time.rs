use atomic_refcell::AtomicRefCell;
use utils::ring_buffer::RingBuffer;

pub use instant::{Duration, Instant};
use once_cell::sync::Lazy;

static TIME_STATE: Lazy<AtomicRefCell<Time>> = Lazy::new(|| AtomicRefCell::new(Time::default()));

// -- Time API

// Tick for the backend
pub(crate) fn tick() {
    TIME_STATE.borrow_mut().tick()
}

/// Average frames per second (calculated using the last 60 frames)
#[inline]
pub fn fps() -> f32 {
    TIME_STATE.borrow().fps()
}

/// Returns an Instant corresponding to now
#[inline]
pub fn now() -> Instant {
    Instant::now()
}

/// Delta time between frames
#[inline]
pub fn delta() -> Duration {
    TIME_STATE.borrow().delta()
}

/// Delta time between frames in seconds
#[inline]
pub fn delta_f32() -> f32 {
    TIME_STATE.borrow().delta_f32()
}

/// Elapsed time since application's init
#[inline]
pub fn elapsed() -> Duration {
    TIME_STATE.borrow().elapsed()
}

/// Elapsed time since application's init in seconds
#[inline]
pub fn elapsed_f32() -> f32 {
    TIME_STATE.borrow().elapsed_f32()
}

/// Application's init time
#[inline]
pub fn init_time() -> Instant {
    TIME_STATE.borrow().init_time()
}

/// Last frame time
#[inline]
pub fn last_time() -> Option<Instant> {
    TIME_STATE.borrow().last_time()
}

/// Measure Application times
#[derive(Debug, Clone)]
pub(crate) struct Time {
    init_time: Instant,
    last_time: Option<Instant>,
    delta: Duration,
    delta_seconds: f32,
    elapsed: Duration,
    elapsed_time: f32,
    fps_cache: RingBuffer<f32, 30>,
    last_cached_fps_time: Instant,
    fps: f32,
}

impl Default for Time {
    fn default() -> Time {
        Time {
            init_time: Instant::now(),
            last_time: None,
            delta: Duration::from_secs(0),
            delta_seconds: 0.0,
            elapsed: Duration::from_secs(0),
            elapsed_time: 0.0,
            fps_cache: Default::default(),
            last_cached_fps_time: Instant::now(),
            fps: 0.0,
        }
    }
}

impl Time {
    #[inline]
    pub(crate) fn tick(&mut self) {
        let now = Instant::now();

        if let Some(last_time) = self.last_time {
            self.delta = now - last_time;
            self.delta_seconds = self.delta.as_secs_f32();
        }

        self.last_time = Some(now);

        self.elapsed = now - self.init_time;
        self.elapsed_time = self.elapsed.as_secs_f32();

        // cache fps each 100ms
        let cache_dt = self.last_cached_fps_time.elapsed().as_secs_f32();
        if cache_dt > 0.1 {
            self.fps_cache.push(self.delta_seconds);
            self.fps = 1.0 / (self.fps_cache.iter().sum::<f32>() / self.fps_cache.len() as f32);
        }
    }

    /// Average frames per second (calculated using the last 60 frames)
    #[inline]
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Delta time between frames
    #[inline]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Delta time between frames in seconds
    #[inline]
    pub fn delta_f32(&self) -> f32 {
        self.delta_seconds
    }

    /// Elapsed time since application's init
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Elapsed time since application's init in seconds
    #[inline]
    pub fn elapsed_f32(&self) -> f32 {
        self.elapsed_time
    }

    /// Application's init time
    #[inline]
    pub fn init_time(&self) -> Instant {
        self.init_time
    }

    /// Last frame time
    #[inline]
    pub fn last_time(&self) -> Option<Instant> {
        self.last_time
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use instant::Duration;

    // Busy spin, to await N time and measure time, because thread::busy_spin is not precise
    fn busy_spin(duration: Duration) {
        let start = Instant::now();
        while start.elapsed() < duration {}
    }

    #[test]
    fn test_time_initialization() {
        let time = Time::default();

        assert!(time.init_time.elapsed().as_secs_f32() >= 0.0);
        assert_eq!(time.last_time, None);
        assert_eq!(time.delta, Duration::from_secs(0));
        assert_eq!(time.delta_seconds, 0.0);
        assert_eq!(time.elapsed, Duration::from_secs(0));
        assert_eq!(time.elapsed_time, 0.0);
        assert_eq!(time.fps, 0.0);
    }

    #[test]
    fn test_tick() {
        let mut time = Time::default();
        time.tick();

        busy_spin(Duration::from_millis(200));
        time.tick();

        assert!(time.delta().as_secs_f32() > 0.0);
        assert!(time.delta_f32() > 0.0);
        assert!(time.elapsed().as_secs_f32() > 0.0);
        assert!(time.elapsed_f32() > 0.0);
        assert!(time.last_time().is_some());
    }

    #[test]
    fn test_delta_time() {
        let mut time = Time::default();
        time.tick();

        // Simulate two frames
        busy_spin(Duration::from_millis(100));
        time.tick();
        let delta1 = time.delta_f32();

        busy_spin(Duration::from_millis(100));
        time.tick();
        let delta2 = time.delta_f32();

        assert!((delta1 - 0.1).abs() < 0.02);
        assert!((delta2 - 0.1).abs() < 0.02);
    }

    #[test]
    fn test_fps_calculation() {
        let mut time = Time::default();
        time.tick();

        // simulate some ticks
        for _ in 0..100 {
            busy_spin(Duration::from_secs_f32(1.0 / 60.0)); // ~60 FPS
            time.tick();
        }

        let fps = time.fps();
        assert!((fps - 60.0).abs() < 5.0); // 5fps as margin of error
    }

    #[test]
    fn test_elapsed_time() {
        let mut time = Time::default();

        busy_spin(Duration::from_millis(200)); // simulate 200ms elapsed
        time.tick();
        let elapsed1 = time.elapsed_f32();

        busy_spin(Duration::from_millis(400)); // simulate another 400ms elapsed
        time.tick();
        let elapsed2 = time.elapsed_f32();

        // Ensure elapsed time is approximately as expected
        assert!((elapsed1 - 0.2).abs() < 0.05); // allowing small margin for test execution delay
        assert!((elapsed2 - 0.6).abs() < 0.05); // allowing small margin for test execution delay
    }
}
