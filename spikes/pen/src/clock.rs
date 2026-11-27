//! CLOCK_MONOTONIC helpers. Wayland input events carry a 32-bit millisecond
//! timestamp; compositors (Hyprland, KWin) take it from CLOCK_MONOTONIC.

pub fn monotonic_ns() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: valid pointer to a timespec on the stack.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Expand a wrapping u32 millisecond timestamp to full CLOCK_MONOTONIC ns,
/// assuming the event happened shortly before `now_ns`.
pub fn reconstruct_ms_timestamp(event_ms: u32, now_ns: u64) -> u64 {
    let now_ms = now_ns / 1_000_000;
    let age_ms = (now_ms as u32).wrapping_sub(event_ms) as u64;
    now_ms.saturating_sub(age_ms) * 1_000_000
}

/// Read an arbitrary POSIX clock (as announced by `wp_presentation.clock_id`).
pub fn clock_ns(clock_id: u32) -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: valid pointer; an invalid clock id just returns an error and leaves ts at 0.
    unsafe { libc::clock_gettime(clock_id as libc::clockid_t, &mut ts) };
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

/// Convert a timestamp on `clock_id` to CLOCK_MONOTONIC by sampling both clocks now.
pub fn to_monotonic(t_ns: u64, clock_id: u32) -> u64 {
    if clock_id == libc::CLOCK_MONOTONIC as u32 {
        return t_ns;
    }
    let a = monotonic_ns();
    let b = clock_ns(clock_id);
    let c = monotonic_ns();
    let offset = ((a / 2 + c / 2) as i128) - b as i128;
    (t_ns as i128 + offset).max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_recent_timestamps_across_wrap() {
        let now_ns = (u32::MAX as u64 + 5) * 1_000_000; // just after a 32-bit wrap
        let event_ms = u32::MAX - 2; // 7 ms earlier, before the wrap
        assert_eq!(
            reconstruct_ms_timestamp(event_ms, now_ns),
            now_ns - 7_000_000
        );
        assert_eq!(reconstruct_ms_timestamp(42, 50 * 1_000_000), 42 * 1_000_000);
    }
}
