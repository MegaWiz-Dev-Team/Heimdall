/// Bounded FIFO admission control for the local single-process backend.
///
/// The MLX/llama.cpp backend batches a healthy number of concurrent
/// sequences, but past its batch/KV ceiling it degrades and then returns
/// 5xx (measured: ~30% errors at concurrency 64 on gemma-4-26b). Forwarding
/// unbounded concurrency turns a capacity problem into client-visible
/// failures. This gate caps in-flight requests to a configurable level and
/// sheds the rest cleanly with an OpenAI-shaped 429/503 instead of letting
/// them pile onto the backend.
///
/// Design mirrors colibrì's `--max-queue` / `--queue-timeout`: a Semaphore of
/// `max_inflight` permits (Tokio's is FIFO) plus a `max_queue` bound on
/// waiters, so total admitted-or-waiting never exceeds
/// `max_inflight + max_queue`. External providers are NOT gated here — they
/// have their own capacity — only the local backend path calls `admit()`.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct Admission {
    sem: Arc<Semaphore>,
    /// running + waiting; bounded by `capacity`
    depth: Arc<AtomicUsize>,
    capacity: usize,
    max_inflight: usize,
    max_queue: usize,
    queue_timeout: Duration,
    enabled: bool,
}

/// Held for the lifetime of the backend call; releases the permit and
/// decrements depth on drop (RAII — correct on every early-return / panic /
/// stream-end path).
pub struct AdmitGuard {
    _permit: OwnedSemaphorePermit,
    depth: Arc<AtomicUsize>,
    pub queue_wait: Duration,
}

impl Drop for AdmitGuard {
    fn drop(&mut self) {
        self.depth.fetch_sub(1, Ordering::SeqCst);
    }
}

pub enum Admitted {
    /// Admitted — hold the guard for the whole backend call.
    Ok(AdmitGuard),
    /// Queue is full (capacity exceeded) → 429, retryable.
    Full,
    /// Waited longer than `queue_timeout` for a slot → 503, retryable.
    Timeout,
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

impl Admission {
    pub fn from_env() -> Self {
        // ADMISSION_ENABLED=0 restores the old unbounded behavior (baseline).
        let enabled = std::env::var("ADMISSION_ENABLED")
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                v != "0" && v != "false" && v != "off"
            })
            .unwrap_or(true);
        let max_inflight = env_usize("MAX_INFLIGHT", 16).max(1);
        let max_queue = env_usize("MAX_QUEUE", 32);
        let queue_timeout = Duration::from_secs(env_usize("QUEUE_TIMEOUT_S", 30) as u64);

        // When disabled, make the gate effectively unbounded so the code path
        // is uniform (one admit() call site) but never rejects or blocks.
        let permits = if enabled { max_inflight } else { Semaphore::MAX_PERMITS };
        let capacity = if enabled { max_inflight + max_queue } else { usize::MAX };

        Self {
            sem: Arc::new(Semaphore::new(permits)),
            depth: Arc::new(AtomicUsize::new(0)),
            capacity,
            max_inflight,
            max_queue,
            queue_timeout,
            enabled,
        }
    }

    pub fn describe(&self) -> String {
        if !self.enabled {
            "disabled (unbounded)".to_string()
        } else {
            format!(
                "max_inflight={} max_queue={} capacity={} queue_timeout={}s",
                self.max_inflight,
                self.max_queue,
                self.capacity,
                self.queue_timeout.as_secs()
            )
        }
    }

    /// Current running+waiting depth (for /health).
    pub fn depth(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Try to admit a request. On `Ok`, hold the returned guard across the
    /// entire backend call (including the streamed body) so the permit is
    /// released only when the response is fully done.
    pub async fn admit(&self) -> Admitted {
        // Reserve a slot in the bounded queue first (cheap, lock-free).
        let prev = self.depth.fetch_add(1, Ordering::SeqCst);
        if prev >= self.capacity {
            self.depth.fetch_sub(1, Ordering::SeqCst);
            metrics::counter!("admission_rejected_total", "reason" => "full").increment(1);
            return Admitted::Full;
        }

        let t0 = Instant::now();
        match tokio::time::timeout(self.queue_timeout, self.sem.clone().acquire_owned()).await {
            Ok(Ok(permit)) => {
                let queue_wait = t0.elapsed();
                metrics::counter!("admission_admitted_total").increment(1);
                metrics::histogram!("admission_queue_wait_seconds")
                    .record(queue_wait.as_secs_f64());
                Admitted::Ok(AdmitGuard {
                    _permit: permit,
                    depth: self.depth.clone(),
                    queue_wait,
                })
            }
            Ok(Err(_closed)) => {
                // Semaphore closed — should never happen; treat as full.
                self.depth.fetch_sub(1, Ordering::SeqCst);
                metrics::counter!("admission_rejected_total", "reason" => "closed").increment(1);
                Admitted::Full
            }
            Err(_elapsed) => {
                self.depth.fetch_sub(1, Ordering::SeqCst);
                metrics::counter!("admission_rejected_total", "reason" => "timeout").increment(1);
                Admitted::Timeout
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn admits_up_to_inflight_then_queues_then_rejects() {
        std::env::set_var("MAX_INFLIGHT", "2");
        std::env::set_var("MAX_QUEUE", "1");
        std::env::set_var("QUEUE_TIMEOUT_S", "10");
        std::env::remove_var("ADMISSION_ENABLED");
        let a = Admission::from_env();

        // 2 in-flight permits + 1 queue slot = capacity 3.
        let g1 = match a.admit().await { Admitted::Ok(g) => g, _ => panic!("g1 should admit") };
        let g2 = match a.admit().await { Admitted::Ok(g) => g, _ => panic!("g2 should admit") };
        assert_eq!(a.depth(), 2);

        // 3rd would occupy the queue slot but block on the permit; 4th must be
        // rejected as Full immediately (capacity exceeded).
        let a2 = a.clone();
        let waiter = tokio::spawn(async move { a2.admit().await });
        tokio::time::sleep(Duration::from_millis(50)).await; // let waiter reserve the queue slot
        assert_eq!(a.depth(), 3);
        assert!(matches!(a.admit().await, Admitted::Full), "4th must be Full");

        drop(g1); // frees a permit → the queued waiter proceeds
        let g3 = match waiter.await.unwrap() { Admitted::Ok(g) => g, _ => panic!("waiter should admit") };
        drop(g2);
        drop(g3);
        assert_eq!(a.depth(), 0);
    }

    #[tokio::test]
    async fn disabled_never_rejects() {
        std::env::set_var("ADMISSION_ENABLED", "0");
        let a = Admission::from_env();
        assert!(!a.enabled());
        let mut guards = Vec::new();
        for _ in 0..100 {
            match a.admit().await {
                Admitted::Ok(g) => guards.push(g),
                _ => panic!("disabled gate must never reject"),
            }
        }
        assert_eq!(a.depth(), 100);
    }
}
