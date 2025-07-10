// Agent API middleware
// Contains middleware for rate limiting, request queuing, and tenant isolation

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use dashmap::DashMap;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Semaphore, SemaphorePermit};
use tower::Service;
use tracing::{debug, error, info, warn};

// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    // Maximum number of requests per time period
    pub max_requests: u32,
    // Time period in seconds
    pub time_period_secs: u32,
    // Maximum queue size
    pub max_queue_size: usize,
    // Max concurrent requests
    pub max_concurrent_requests: usize,
    // Whether to use tenant-specific limits or global limits
    pub tenant_specific: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            time_period_secs: 60,
            max_queue_size: 50,
            max_concurrent_requests: 20,
            tenant_specific: true,
        }
    }
}

// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    tokens: u32,
    max_tokens: u32,
    refill_interval: Duration,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u32, refill_interval_secs: u32) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_interval: Duration::from_secs(refill_interval_secs as u64),
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if elapsed >= self.refill_interval {
            let refills = elapsed.as_secs() as u32 / self.refill_interval.as_secs() as u32;
            let new_tokens = self.tokens + (refills * self.max_tokens);
            self.tokens = std::cmp::min(new_tokens, self.max_tokens);
            self.last_refill = now;
        }
    }
}

// Request queue for handling bursts
#[derive(Debug)]
struct RequestQueue<T> {
    queue: VecDeque<T>,
    max_size: usize,
}

impl<T> RequestQueue<T> {
    fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn enqueue(&mut self, item: T) -> bool {
        if self.queue.len() < self.max_size {
            self.queue.push_back(item);
            true
        } else {
            false
        }
    }

    fn dequeue(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// Main rate limiter structure
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    tenant_buckets: DashMap<String, Arc<Mutex<TokenBucket>>>,
    global_bucket: Arc<Mutex<TokenBucket>>,
    queue_semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config: config.clone(),
            tenant_buckets: DashMap::new(),
            global_bucket: Arc::new(Mutex::new(TokenBucket::new(
                config.max_requests,
                config.time_period_secs,
            ))),
            queue_semaphore: Arc::new(Semaphore::new(config.max_concurrent_requests)),
        }
    }

    // Get a bucket for a specific tenant, or create one if it doesn't exist
    async fn get_tenant_bucket(&self, tenant_id: &str) -> Arc<Mutex<TokenBucket>> {
        if let Some(bucket) = self.tenant_buckets.get(tenant_id) {
            bucket.value().clone()
        } else {
            let bucket = Arc::new(Mutex::new(TokenBucket::new(
                self.config.max_requests,
                self.config.time_period_secs,
            )));
            self.tenant_buckets
                .insert(tenant_id.to_string(), bucket.clone());
            bucket
        }
    }

    // Try to acquire a token, either from tenant-specific bucket or global bucket
    async fn try_acquire(&self, tenant_id: Option<&str>) -> bool {
        if self.config.tenant_specific && tenant_id.is_some() {
            let tenant_id = tenant_id.unwrap();
            let bucket_guard = self.get_tenant_bucket(tenant_id).await;
            let mut bucket = bucket_guard.lock().await;
            bucket.try_acquire()
        } else {
            let mut bucket = self.global_bucket.lock().await;
            bucket.try_acquire()
        }
    }

    // Acquire a semaphore permit for concurrent request limiting
    async fn acquire_permit(&self) -> Option<SemaphorePermit> {
        match self.queue_semaphore.acquire().await {
            Ok(permit) => Some(permit),
            Err(_) => None,
        }
    }
}

// Extract tenant ID from request
pub struct TenantId(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for TenantId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try to extract from header
        if let Some(tenant_id) = parts.headers.get("x-tenant-id") {
            if let Ok(tenant_id) = tenant_id.to_str() {
                return Ok(TenantId(tenant_id.to_string()));
            }
        }

        // Try to extract from path
        if let Some(path_parts) = parts.uri.path().split('/').collect::<Vec<_>>().get(2) {
            if *path_parts == "tenant" {
                if let Some(tenant_id) = parts.uri.path().split('/').collect::<Vec<_>>().get(3) {
                    return Ok(TenantId(tenant_id.to_string()));
                }
            }
        }

        // Try to extract from query parameters
        if let Some(query) = parts.uri.query() {
            for pair in query.split('&') {
                let mut parts = pair.split('=');
                if let (Some("tenant_id"), Some(tenant_id)) = (parts.next(), parts.next()) {
                    return Ok(TenantId(tenant_id.to_string()));
                }
            }
        }

        // Default tenant if not found
        Ok(TenantId("default".to_string()))
    }
}

// Extract tenant ID from request body
pub async fn extract_tenant_from_body(json: &Value) -> Option<String> {
    // Try to get from context.tenant_id field
    if let Some(tenant_id) = json
        .get("context")
        .and_then(|ctx| ctx.get("tenant_id"))
        .and_then(|id| id.as_str())
    {
        return Some(tenant_id.to_string());
    }

    // Try to get from user_context
    if let Some(tenant_id) = json
        .get("context")
        .and_then(|ctx| ctx.get("user_context"))
        .and_then(|user| user.get("tenant_id"))
        .and_then(|id| id.as_str())
    {
        return Some(tenant_id.to_string());
    }

    None
}

// Rate limiting middleware for Axum
pub async fn rate_limit(
    State(rate_limiter): State<Arc<RateLimiter>>,
    tenant_id: TenantId,
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    debug!("Rate limiting request for tenant: {}", tenant_id.0);

    // Try to acquire a token
    let is_allowed = rate_limiter.try_acquire(Some(&tenant_id.0)).await;

    if !is_allowed {
        // Request exceeds rate limit
        warn!("Rate limit exceeded for tenant: {}", tenant_id.0);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.",
        )
            .into_response();
    }

    // Try to acquire a semaphore permit for concurrent request limiting
    let permit = match rate_limiter.acquire_permit().await {
        Some(permit) => permit,
        None => {
            error!("Too many concurrent requests for tenant: {}", tenant_id.0);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Server is processing too many requests. Please try again later.",
            )
                .into_response();
        }
    };

    // Process the request
    let response = next.run(request).await;

    // The permit is automatically dropped when it goes out of scope,
    // releasing the semaphore slot for the next request
    drop(permit);

    response
}

// Queue-aware rate limiting middleware future
pub struct QueuedRequest<F> {
    future: F,
    _permit: SemaphorePermit<'static>,
}

impl<F: Future<Output = Response> + Unpin> Future for QueuedRequest<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx)
    }
}

// Request validation middleware
pub async fn validate_tenant(
    tenant_id: TenantId,
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    // In a real implementation, this would validate the tenant against a database
    // or authentication service to ensure the request is authorized

    // For now, we'll just log the tenant and continue
    debug!("Request for tenant: {}", tenant_id.0);

    // Continue with the request
    next.run(request).await
}
