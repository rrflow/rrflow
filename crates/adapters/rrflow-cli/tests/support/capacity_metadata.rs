//! Bounded metadata creation with a fresh native failure after settlement.
use super::{capacity_code, Duration, Instant};
use std::io;

pub(super) fn fill<C>(
    context: &mut C,
    started: Instant,
    count: impl Fn(&C) -> usize,
    mut create: impl FnMut(&mut C, usize) -> io::Result<()>,
    mut settle: impl FnMut(&mut C) -> io::Result<()>,
) -> io::Result<i32> {
    let mut settled_since_create = false;
    let mut creates = 0_u64;
    let mut barriers = 0_u64;
    let mut create_us = 0_u128;
    let mut settle_us = 0_u128;
    loop {
        let before = count(context);
        assert!(before < 8192, "metadata entry bound");
        assert!(
            started.elapsed() < Duration::from_secs(25),
            "bounded metadata fill: files={before} creates={creates} barriers={barriers} create_us={create_us} settle_us={settle_us} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let operation = Instant::now();
        let result = create(context, before);
        creates += 1;
        create_us += operation.elapsed().as_micros();
        match result {
            Ok(()) => {
                assert_eq!(
                    count(context),
                    before + 1,
                    "metadata ownership registration"
                );
                settled_since_create = false;
                if !count(context).is_multiple_of(128) {
                    continue;
                }
            }
            Err(error) => {
                let Some(code) = error.raw_os_error().filter(|code| capacity_code(*code)) else {
                    return Err(error);
                };
                assert_eq!(
                    count(context),
                    before,
                    "failed create changed metadata ownership"
                );
                if settled_since_create {
                    eprintln!("native_capacity_metadata_phase={{\"files\":{before},\"creates\":{creates},\"barriers\":{barriers},\"create_us\":{create_us},\"settle_us\":{settle_us}}}");
                    return Ok(code);
                }
            }
        }
        // A partial batch and the first capacity failure need settlement too.
        // Retry the same next name; a barrier may make more allocation available.
        let operation = Instant::now();
        let result = settle(context);
        barriers += 1;
        settle_us += operation.elapsed().as_micros();
        result?;
        settled_since_create = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Allocation {
        files: usize,
        available: usize,
        settlements: usize,
        release_on_settlement: usize,
    }

    fn full() -> io::Error {
        io::Error::from_raw_os_error(if cfg!(windows) { 112 } else { 28 })
    }

    fn create(state: &mut Allocation, index: usize) -> io::Result<()> {
        assert_eq!(state.files, index);
        if state.available == 0 {
            return Err(full());
        }
        state.available -= 1;
        state.files += 1;
        Ok(())
    }

    fn settle(state: &mut Allocation) -> io::Result<()> {
        state.settlements += 1;
        state.available += std::mem::take(&mut state.release_on_settlement);
        Ok(())
    }

    #[test]
    fn metadata_batches_keep_partial_settlement_and_consume_delayed_allocation() {
        let mut state = Allocation {
            available: 693,
            release_on_settlement: 17,
            ..Default::default()
        };
        let code = fill(&mut state, Instant::now(), |s| s.files, create, settle).unwrap();
        assert!(capacity_code(code));
        assert_eq!(state.files, 710);
        assert_eq!(state.available, 0);
        assert_eq!(
            state.settlements, 6,
            "per-file barriers defeat bounded batching"
        );
    }

    #[test]
    fn metadata_first_capacity_error_is_not_exhaustion_before_settlement() {
        let mut state = Allocation {
            release_on_settlement: 3,
            ..Default::default()
        };
        fill(&mut state, Instant::now(), |s| s.files, create, settle).unwrap();
        assert_eq!(state.files, 3);
        assert_eq!(state.settlements, 2);
    }

    #[test]
    fn metadata_noncapacity_and_settlement_errors_are_not_exhaustion() {
        let mut state = Allocation::default();
        let error = fill(
            &mut state,
            Instant::now(),
            |s| s.files,
            |_, _| Err(io::Error::from_raw_os_error(5)),
            settle,
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(5));
        assert_eq!(state.settlements, 0);
        // A barrier error alone cannot stand in for a later failed create.
        let error = fill(
            &mut state,
            Instant::now(),
            |s| s.files,
            create,
            |_| Err(full()),
        )
        .unwrap_err();
        assert!(capacity_code(error.raw_os_error().unwrap()));
    }

    #[test]
    fn metadata_expired_clock_and_entry_bound_prevent_further_effects() {
        for (files, started) in [
            (0, Instant::now() - Duration::from_secs(25)),
            (8192, Instant::now()),
        ] {
            let mut state = Allocation {
                files,
                available: 1,
                ..Default::default()
            };
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = fill(&mut state, started, |s| s.files, create, settle);
            }))
            .is_err());
            assert_eq!(state.available, 1);
            assert_eq!(state.settlements, 0);
        }
    }
}
