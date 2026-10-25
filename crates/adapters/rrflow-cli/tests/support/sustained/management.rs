//! One owned, deadline-bound management task; polling never waits for completion.
use super::{DriverResult, Instant};
use std::future::Future;
use tokio::task::JoinSet;

pub(super) struct ManagementTask<T> {
    tasks: JoinSet<DriverResult<T>>,
}

impl<T: Send + 'static> ManagementTask<T> {
    pub(super) fn new() -> Self {
        Self {
            tasks: JoinSet::new(),
        }
    }

    pub(super) fn is_pending(&self) -> bool {
        !self.tasks.is_empty()
    }

    pub(super) fn start(
        &mut self,
        deadline: Instant,
        work: impl Future<Output = DriverResult<T>> + Send + 'static,
    ) -> DriverResult<()> {
        if self.is_pending() {
            return Err("management task capacity exhausted".into());
        }
        if Instant::now() >= deadline {
            return Err("backup interference deadline".into());
        }
        self.tasks.spawn(async move {
            let result = tokio::time::timeout_at(deadline.into(), work)
                .await
                .map_err(|_| "backup interference deadline".to_owned())?;
            if Instant::now() > deadline {
                return Err("backup interference deadline".into());
            }
            result
        });
        Ok(())
    }

    pub(super) fn try_finish(&mut self) -> DriverResult<Option<T>> {
        self.tasks
            .try_join_next()
            .map(|result| result.map_err(|_| "backup task panicked".to_owned())?)
            .transpose()
    }

    pub(super) async fn finish(&mut self) -> DriverResult<Option<T>> {
        self.tasks
            .join_next()
            .await
            .map(|result| result.map_err(|_| "backup task panicked".to_owned())?)
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };

    #[tokio::test]
    async fn unfinished_management_does_not_block_next_offer_or_allow_a_second_task() {
        let (release, wait) = tokio::sync::oneshot::channel();
        let mut task = ManagementTask::new();
        task.start(Instant::now() + Duration::from_secs(2), async move {
            wait.await.map_err(|_| "cancelled".to_owned())?;
            Ok(7)
        })
        .unwrap();
        tokio::task::yield_now().await;
        assert!(task.try_finish().unwrap().is_none());
        assert!(task
            .start(Instant::now() + Duration::from_secs(2), async { Ok(8) })
            .is_err());
        // A following offered operation runs while the management work is held.
        let offered = tokio::time::timeout(Duration::from_millis(100), async { 11 })
            .await
            .unwrap();
        assert_eq!(offered, 11);
        release.send(()).unwrap();
        assert_eq!(task.finish().await.unwrap(), Some(7));
        assert!(!task.is_pending());
        assert_eq!(task.try_finish().unwrap(), None);
    }

    #[tokio::test]
    async fn management_uses_the_original_deadline_and_preserves_failure() {
        let mut task = ManagementTask::<u8>::new();
        assert!(task
            .start(
                Instant::now() - Duration::from_secs(1),
                std::future::pending()
            )
            .unwrap_err()
            .contains("deadline"));
        task.start(
            Instant::now() + Duration::from_millis(10),
            std::future::pending(),
        )
        .unwrap();
        assert!(task.finish().await.unwrap_err().contains("deadline"));
        task.start(Instant::now() + Duration::from_secs(1), async {
            Err("archive failed".into())
        })
        .unwrap();
        assert_eq!(task.finish().await.unwrap_err(), "archive failed");
        assert!(!task.is_pending());
    }

    #[tokio::test]
    async fn dropping_management_cancels_its_owned_work() {
        struct Release(Arc<AtomicBool>);
        impl Drop for Release {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let released = Arc::new(AtomicBool::new(false));
        let owner = Release(released.clone());
        let mut task = ManagementTask::<u8>::new();
        task.start(Instant::now() + Duration::from_secs(2), async move {
            let _owner = owner;
            std::future::pending().await
        })
        .unwrap();
        tokio::task::yield_now().await;
        drop(task);
        tokio::time::timeout(Duration::from_secs(1), async {
            while !released.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
