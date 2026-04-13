use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter: true,
        }
    }
}

impl RetryConfig {
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
            jitter: true,
        }
    }

    pub fn without_jitter(mut self) -> Self {
        self.jitter = false;
        self
    }
}

pub fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let base = config.base_delay_ms as f64;
    let multiplier = 2_f64.powi(attempt as i32);
    let delay = (base * multiplier).min(config.max_delay_ms as f64) as u64;

    let jitter_amount = if config.jitter {
        let pseudo_random = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64)
            .wrapping_mul(attempt as u64 + 1);
        (delay as f64 * 0.25 * (pseudo_random % 100) as f64 / 100.0) as u64
    } else {
        0
    };

    Duration::from_millis(delay + jitter_amount)
}

pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut f: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut last_err = None;
    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt >= config.max_retries {
                    return Err(e);
                }
                last_err = Some(e);
                let delay = calculate_delay(attempt, &config);
                tokio::time::sleep(delay).await;
            }
        }
    }
    Err(last_err.unwrap())
}

pub async fn retry_api_call<F, Fut, T>(
    config: RetryConfig,
    mut f: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempt = 0u32;
    let mut last_error: Option<anyhow::Error> = None;

    while attempt <= config.max_retries {
        attempt += 1;
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                let err_str = e.to_string();
                let is_network = err_str.contains("connection") || err_str.contains("timeout") || err_str.contains("network");
                let is_server = err_str.contains("500") || err_str.contains("502") || err_str.contains("503");
                let is_rate_limit = err_str.contains("429") || err_str.contains("rate limit") || err_str.contains("529");

                if !is_network && !is_server && !is_rate_limit {
                    return Err(e);
                }

                if attempt > config.max_retries {
                    return Err(e);
                }

                last_error = Some(e);
                let delay = calculate_delay(attempt - 1, &config);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Retry exhausted")))
}
