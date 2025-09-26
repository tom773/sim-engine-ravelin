use std::time::SystemTime;

#[cfg(target_arch = "wasm32")]
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(target_arch = "wasm32")]
fn base_clock() -> &'static (Instant, SystemTime) {
    use std::time::UNIX_EPOCH;

    static BASE: OnceLock<(Instant, SystemTime)> = OnceLock::new();
    BASE.get_or_init(|| {
        let millis = js_sys::Date::now();
        let duration = Duration::from_secs_f64(millis / 1_000.0);
        let base_system = UNIX_EPOCH + duration;
        (Instant::now(), base_system)
    })
}

#[cfg(target_arch = "wasm32")]
pub fn wall_clock_now() -> SystemTime {
    let (instant, base) = base_clock();
    base.checked_add(instant.elapsed()).unwrap_or(*base)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wall_clock_now() -> SystemTime {
    SystemTime::now()
}
