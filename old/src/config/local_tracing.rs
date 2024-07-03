use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub fn initialize_tracing_subscriber() {
    let filter = EnvFilter::new("info,intelli=trace");

    let subscriber = FmtSubscriber::builder()
        .with_ansi(true)
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(filter)
        .compact()
        .finish();

    LogTracer::init().unwrap();
    tracing::subscriber::set_global_default(subscriber).unwrap();
}
