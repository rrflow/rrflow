use super::filler::*;
use super::*;
use std::cell::Cell;

#[test]
fn successful_small_write_recovers_bulk_allocation_without_bytewise_drain() {
    struct ReopenedAllocation {
        written: usize,
        calls: usize,
    }
    const AVAILABLE: usize = 1024 * 1024 + 1;
    impl Write for ReopenedAllocation {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.calls += 1;
            assert!(
                self.calls < 128,
                "available allocation drained byte by byte"
            );
            if self.written == AVAILABLE || (self.written == 0 && bytes.len() > 1) {
                return Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                    112
                } else {
                    28
                }));
            }
            let count = bytes.len().min(AVAILABLE - self.written);
            self.written += count;
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut file = ReopenedAllocation {
        written: 0,
        calls: 0,
    };
    let mut syncs = 0;
    let (bytes, code) = fill_data(&mut file, 2 * 1024 * 1024, Instant::now(), |_, _| {
        syncs += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(bytes, AVAILABLE as u64);
    assert_eq!(syncs, 1);
    assert!(capacity_code(code));
}

#[test]
fn settle_pressure_repeats_after_either_category_allocates() {
    let mut calls = 0;
    let states = [(8, 1), (8, 2), (12, 2), (12, 2)];
    settle_pressure(Instant::now(), (0, 0), || {
        let state = states[calls];
        calls += 1;
        state
    });
    assert_eq!(
        calls,
        states.len(),
        "stopped before both categories settled"
    );
}

#[test]
fn settle_pressure_stable_state_stops_and_expired_clock_has_no_effect() {
    let calls = Cell::new(0);
    settle_pressure(Instant::now(), (8, 2), || {
        calls.set(calls.get() + 1);
        (8, 2)
    });
    assert_eq!(calls.get(), 1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        settle_pressure(expired_fill_clock(), (8, 2), || {
            calls.set(calls.get() + 1);
            (8, 2)
        });
    }))
    .is_err());
    assert_eq!(
        calls.get(),
        1,
        "expired original clock allowed a new effect"
    );
}

#[test]
fn metadata_payload_is_charged_before_effect_and_reduces_data_allowance() {
    let mut charged = 0;
    let calls = Cell::new(0);
    for code in [28, 5] {
        let error = allocate_metadata_payload(&mut charged, 8, 16, 3, || {
            calls.set(calls.get() + 1);
            Err(std::io::Error::from_raw_os_error(code))
        })
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(code));
    }
    assert_eq!(charged, 6, "failed native effects retain their allowance");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = allocate_metadata_payload(&mut charged, 8, 16, 3, || {
            calls.set(calls.get() + 1);
            Ok(())
        });
    }))
    .is_err());
    assert_eq!(calls.get(), 2, "over-budget metadata must not reach the OS");
    assert_eq!(charged, 6);
    let mut file = DelayedAllocation::new();
    let mut data_bytes = 8;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fill_remaining_data(
            &mut file,
            &mut data_bytes,
            16 - charged,
            Instant::now(),
            |_, _| Ok(()),
        );
    }))
    .is_err());
    assert_eq!(
        file.remaining.get(),
        6,
        "data refill ignored metadata charge"
    );
}

#[test]
fn capacity_file_error_still_settles_volume_and_preserves_errors() {
    let capacity = if cfg!(windows) { 112 } else { 28 };
    for file_code in [None, Some(capacity), Some(5)] {
        for volume_code in [None, Some(capacity), Some(5)] {
            let calls = Cell::new(0);
            let result = finish_allocation_barrier(
                file_code.map_or(Ok(()), |code| Err(std::io::Error::from_raw_os_error(code))),
                || {
                    calls.set(calls.get() + 1);
                    volume_code.map_or(Ok(()), |code| Err(std::io::Error::from_raw_os_error(code)))
                },
            );
            assert_eq!(calls.get(), u32::from(file_code != Some(5)));
            let expected = if file_code == Some(5) {
                file_code
            } else {
                volume_code.or(file_code)
            };
            assert_eq!(
                result.err().and_then(|error| error.raw_os_error()),
                expected
            );
        }
    }
}

struct DelayedAllocation {
    remaining: Cell<usize>,
    syncs: Cell<usize>,
    write_error: i32,
}
impl DelayedAllocation {
    fn new() -> Self {
        Self {
            remaining: Cell::new(8),
            syncs: Cell::new(0),
            write_error: if cfg!(windows) { 112 } else { 28 },
        }
    }
}
impl Write for DelayedAllocation {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = bytes.len().min(self.remaining.get());
        if n == 0 {
            return Err(std::io::Error::from_raw_os_error(self.write_error));
        }
        self.remaining.set(self.remaining.get() - n);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn stable_full_refill_uses_one_byte_before_and_after_settlement() {
    struct Full {
        attempts: Vec<usize>,
    }
    impl Write for Full {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.attempts.push(bytes.len());
            Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                112
            } else {
                28
            }))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut file = Full {
        attempts: Vec::new(),
    };
    let mut allocated = 128;
    let mut barriers = Vec::new();
    let code = fill_remaining_data(
        &mut file,
        &mut allocated,
        1024,
        Instant::now(),
        |_, reason| {
            barriers.push(reason);
            Ok(())
        },
    )
    .unwrap();
    assert!(capacity_code(code));
    assert_eq!(allocated, 128);
    assert_eq!(barriers, [FillBarrier::Capacity]);
    assert_eq!(
        file.attempts,
        [1, 1],
        "a stable refill must not repeat bulk-sized failed probes"
    );
}

#[test]
fn refill_consumes_space_released_by_settlement_before_final_denial() {
    struct Refill {
        remaining: Cell<usize>,
        attempts: Vec<usize>,
    }
    impl Write for Refill {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.attempts.push(bytes.len());
            let count = bytes.len().min(self.remaining.get());
            if count == 0 {
                return Err(std::io::Error::from_raw_os_error(if cfg!(windows) {
                    112
                } else {
                    28
                }));
            }
            self.remaining.set(self.remaining.get() - count);
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut file = Refill {
        remaining: Cell::new(0),
        attempts: Vec::new(),
    };
    let mut allocated = 0;
    let mut barriers = 0;
    let code = fill_remaining_data(
        &mut file,
        &mut allocated,
        128,
        Instant::now(),
        |file, reason| {
            assert_eq!(reason, FillBarrier::Capacity);
            if barriers == 0 {
                file.remaining.set(17);
            }
            barriers += 1;
            Ok(())
        },
    )
    .unwrap();
    assert!(capacity_code(code));
    assert_eq!(allocated, 17);
    assert_eq!(file.remaining.get(), 0);
    assert_eq!(barriers, 2);
    assert_eq!(&file.attempts[..2], &[1, 1]);
    assert!(
        file.attempts[2] > 1,
        "successful probe must resume bulk writes"
    );
    assert_eq!(file.attempts.last(), Some(&1));
}

#[test]
fn metadata_refills_share_the_original_byte_and_time_bounds() {
    let mut file = DelayedAllocation::new();
    let mut allocated = 0;
    let started = Instant::now();
    let code = fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(())).unwrap();
    assert!(capacity_code(code));
    assert_eq!(allocated, 8);
    file.remaining.set(8); // Allocation released by a later metadata transaction.
    fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(())).unwrap();
    assert_eq!(allocated, 16);
    file.remaining.set(8);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fill_remaining_data(&mut file, &mut allocated, 20, started, |_, _| Ok(()));
    }))
    .is_err());
    assert_eq!(
        file.remaining.get(),
        4,
        "refill exceeded cumulative allowance"
    );
    let expired = expired_fill_clock();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fill_remaining_data(&mut file, &mut allocated, 20, expired, |_, _| {
            panic!("expired refill synchronized")
        });
    }))
    .is_err());
    assert_eq!(file.remaining.get(), 4, "refill restarted the clock");
}

#[test]
fn disposable_apfs_fill_defers_intervals_but_settles_and_drains_delayed_allocation() {
    const INITIAL: usize = 60 * 4 * 1024 * 1024;
    for periodic_file_sync in [false, true] {
        let mut file = DelayedAllocation::new();
        file.remaining.set(INITIAL);
        let file_syncs = Cell::new(0);
        let volume_syncs = Cell::new(0);
        let (bytes, code) = fill_data(
            &mut file,
            (INITIAL + 1024) as u64,
            Instant::now(),
            |file, reason| {
                synchronize_allocation(
                    reason,
                    periodic_file_sync,
                    || {
                        file_syncs.set(file_syncs.get() + 1);
                        Ok(())
                    },
                    || {
                        if volume_syncs.get() == 0 {
                            file.remaining.set(16);
                        }
                        volume_syncs.set(volume_syncs.get() + 1);
                        Ok(())
                    },
                )
            },
        )
        .unwrap();
        assert_eq!(bytes, (INITIAL + 16) as u64);
        assert_eq!(
            file.remaining.get(),
            0,
            "settled allocation escaped pressure"
        );
        assert!(capacity_code(code));
        assert_eq!(volume_syncs.get(), 2, "final settlement was skipped");
        assert_eq!(
            file_syncs.get(),
            if periodic_file_sync { 62 } else { 2 },
            "disposable APFS intervals consumed unnecessary barriers"
        );
    }
}

#[test]
fn exact_interval_still_requires_capacity_settlement_and_fresh_denial() {
    const INTERVAL: usize = 4 * 1024 * 1024;
    let mut file = DelayedAllocation::new();
    file.remaining.set(INTERVAL);
    let mut reasons = Vec::new();
    let (bytes, code) = fill_data(
        &mut file,
        (INTERVAL + 1024) as u64,
        Instant::now(),
        |file, reason| {
            reasons.push(reason);
            if reason == FillBarrier::Capacity && file.syncs.get() == 0 {
                file.remaining.set(16);
                file.syncs.set(1);
            }
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(bytes, (INTERVAL + 16) as u64);
    assert_eq!(file.remaining.get(), 0);
    assert_eq!(
        reasons,
        [
            FillBarrier::Interval,
            FillBarrier::Capacity,
            FillBarrier::Capacity
        ]
    );
    assert!(capacity_code(code));
}

#[test]
fn settles_pending_allocation_before_declaring_full() {
    // Both a successful barrier and delayed allocation failure can change
    // what the next write observes. Neither alone proves settled capacity.
    for delayed_error in [false, true] {
        let mut file = DelayedAllocation::new();
        let (bytes, code) = fill_data(&mut file, 1024, Instant::now(), |file, _| {
            let first = file.syncs.get() == 0;
            if first {
                file.remaining.set(16);
            }
            file.syncs.set(file.syncs.get() + 1);
            if first && delayed_error {
                Err(std::io::Error::from_raw_os_error(file.write_error))
            } else {
                Ok(())
            }
        })
        .unwrap();
        assert_eq!(
            bytes, 24,
            "must consume allocation made available by the final sync"
        );
        assert_eq!(file.remaining.get(), 0);
        assert_eq!(file.syncs.get(), 2);
        assert!(capacity_code(code));
    }
}

#[test]
fn preserves_noncapacity_write_and_sync_errors() {
    for fail_write in [false, true] {
        let mut file = DelayedAllocation::new();
        if fail_write {
            file.write_error = 5;
        }
        let error = fill_data(&mut file, 1024, Instant::now(), |_, _| {
            assert!(
                !fail_write,
                "unexpected write error must not trigger sync/retry"
            );
            Err(std::io::Error::from_raw_os_error(5))
        })
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(5));
        assert!(!capacity_code(5));
    }
}

#[test]
fn cannot_exceed_byte_cap_or_restart_expired_deadline() {
    let mut sink = Vec::new();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fill_data(&mut sink, 13, Instant::now(), |_, _| Ok(()));
    }))
    .is_err());
    assert_eq!(sink.len(), 13);
    let expired = expired_fill_clock();
    let mut file = DelayedAllocation::new();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fill_data(&mut file, 1024, expired, |_, _| {
            panic!("expired fill synchronized")
        });
    }))
    .is_err());
    assert_eq!(file.remaining.get(), 8, "expired fill wrote data");
}
