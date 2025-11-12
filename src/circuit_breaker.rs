use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests are allowed
    Closed,
    /// Failing - requests are rejected immediately
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

/// Circuit breaker error type
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open, request rejected
    Open,
    /// Inner operation failed
    Inner(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::Open => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::Inner(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CircuitBreakerError::Open => None,
            CircuitBreakerError::Inner(e) => Some(e),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes in HalfOpen to close circuit
    pub success_threshold: u32,
    /// Time to wait before transitioning from Open to HalfOpen
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

/// Circuit breaker implementation for protecting operations
#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    config: CircuitBreakerConfig,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        log::info!(
            "Initializing circuit breaker: failure_threshold={}, success_threshold={}, timeout={}s",
            config.failure_threshold,
            config.success_threshold,
            config.timeout.as_secs()
        );

        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            config,
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Execute an operation with circuit breaker protection
    pub async fn call<F, Fut, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        // Check if circuit is open
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                // Check if timeout has passed
                let last_failure = self.last_failure_time.read().await;
                if let Some(time) = *last_failure {
                    if time.elapsed() > self.config.timeout {
                        drop(last_failure);
                        // Transition to half-open
                        log::info!("Circuit breaker transitioning from Open to HalfOpen");
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                    } else {
                        // Still in timeout period, reject request
                        return Err(CircuitBreakerError::Open);
                    }
                }
            }
            _ => {}
        }

        // Execute operation
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(CircuitBreakerError::Inner(e))
            }
        }
    }

    /// Handle successful operation
    async fn on_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                log::debug!(
                    "Circuit breaker HalfOpen success: {}/{}",
                    *success_count,
                    self.config.success_threshold
                );

                if *success_count >= self.config.success_threshold {
                    drop(success_count);
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    log::info!("Circuit breaker closed - service recovered");
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }

    /// Handle failed operation
    async fn on_failure(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::HalfOpen => {
                // Failure in HalfOpen -> back to Open
                drop(state);
                log::warn!("Circuit breaker reopening after failure in HalfOpen state");
                *self.state.write().await = CircuitState::Open;
                *self.last_failure_time.write().await = Some(Instant::now());
                *self.success_count.write().await = 0;
            }
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                log::debug!(
                    "Circuit breaker failure count: {}/{}",
                    *failure_count,
                    self.config.failure_threshold
                );

                if *failure_count >= self.config.failure_threshold {
                    drop(failure_count);
                    drop(state);
                    log::warn!(
                        "Circuit breaker opening after {} consecutive failures",
                        self.config.failure_threshold
                    );
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure_time.write().await = Some(Instant::now());
                }
            }
            _ => {}
        }
    }

    /// Reset circuit breaker to closed state (for testing/manual intervention)
    #[allow(dead_code)]
    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        *self.last_failure_time.write().await = None;
        log::info!("Circuit breaker manually reset to Closed state");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::new(config);

        assert_eq!(cb.state().await, CircuitState::Closed);

        // Successful operations should keep circuit closed
        let result = cb
            .call(|| async { Ok::<_, String>("success") })
            .await;
        assert!(result.is_ok());
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::new(config);

        // Fail 3 times to reach threshold
        for i in 1..=3 {
            let result = cb
                .call(|| async { Err::<(), _>(format!("error {}", i)) })
                .await;
            assert!(result.is_err());
        }

        assert_eq!(cb.state().await, CircuitState::Open);

        // Next call should be rejected immediately
        let result = cb
            .call(|| async { Ok::<_, String>("success") })
            .await;
        assert!(matches!(result, Err(CircuitBreakerError::Open)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb
                .call(|| async { Err::<(), _>("error") })
                .await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Next call should transition to HalfOpen
        let result = cb
            .call(|| async { Ok::<_, String>("success") })
            .await;
        assert!(result.is_ok());
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<(), _>("error") }).await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Two successful calls in HalfOpen should close circuit
        for _ in 0..2 {
            let result = cb
                .call(|| async { Ok::<_, String>("success") })
                .await;
            assert!(result.is_ok());
        }

        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<(), _>("error") }).await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // One success
        let _ = cb.call(|| async { Ok::<_, String>("success") }).await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Failure in HalfOpen should reopen circuit
        let _ = cb.call(|| async { Err::<(), _>("error") }).await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_secs(10),
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<(), _>("error") }).await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);

        // Reset should close circuit immediately
        cb.reset().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Should accept requests now
        let result = cb
            .call(|| async { Ok::<_, String>("success") })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_concurrent_calls() {
        let config = CircuitBreakerConfig {
            failure_threshold: 10,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = Arc::new(CircuitBreaker::new(config));
        let counter = Arc::new(AtomicU32::new(0));

        // Spawn multiple concurrent operations
        let mut handles = vec![];
        for _ in 0..20 {
            let cb_clone = cb.clone();
            let counter_clone = counter.clone();
            let handle = tokio::spawn(async move {
                let _ = cb_clone
                    .call(|| async {
                        counter_clone.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, String>("success")
                    })
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All should succeed
        assert_eq!(counter.load(Ordering::SeqCst), 20);
        assert_eq!(cb.state().await, CircuitState::Closed);
    }
}
