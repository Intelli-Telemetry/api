use std::{sync::Arc, time::Instant};

use ahash::AHashMap;
use compact_str::CompactString;
use ntex::{
    service::{Middleware, Service, ServiceCtx},
    web::{Error, WebRequest, WebResponse},
};
use parking_lot::Mutex;
use tracing::{info, warn};

use crate::error::CommonError;

const RATE_LIMIT: usize = 5;
const RATE_LIMIT_DURATION: u64 = 120;

pub struct LoginLimit;

impl<S> Middleware<S> for LoginLimit {
    type Service = LoginLimitMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        let visitors = Arc::from(Mutex::from(AHashMap::with_capacity(100)));
        LoginLimitMiddleware { service, visitors }
    }
}

pub struct LoginLimitMiddleware<S> {
    service: S,
    visitors: Arc<Mutex<AHashMap<CompactString, (usize, Instant)>>>,
}

impl<S, Err> Service<WebRequest<Err>> for LoginLimitMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error>,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_poll_ready!(service);
    ntex::forward_poll_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        let ip = req.headers().get("CF-Connecting-IP");

        // Only rate limit if the request is coming from the cloudflare proxy
        if let Some(ip) = ip {
            let now = Instant::now();
            let ip = CompactString::from(ip.to_str().unwrap());
            let mut visitors = self.visitors.lock();
            let count = visitors.entry(ip.clone()).or_insert((0, now));

            if now.duration_since(count.1).as_secs() > RATE_LIMIT_DURATION {
                count.0 = 0;
                count.1 = now;
            }

            if count.0 > RATE_LIMIT {
                info!("{} is rate limited", ip);
                return Err(CommonError::RateLimited)?;
            }

            count.0 += 1;
        } else {
            warn!("No CF-Connecting-IP header, not rate limiting");
        }

        let res = ctx.call(&self.service, req).await?;
        Ok(res)
    }
}
