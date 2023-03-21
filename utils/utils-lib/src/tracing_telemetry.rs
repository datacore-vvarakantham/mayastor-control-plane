/// OpenTelemetry KeyVal for Processor Tags
pub use opentelemetry::{global, trace, Context, KeyValue};

use opentelemetry::sdk::{propagation::TraceContextPropagator, Resource};
use tracing::Level;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

use crate::{nats::TypedNats, publisher::EventHandle};

/// Parse KeyValues from structopt's cmdline arguments
pub fn parse_key_value(source: &str) -> Result<KeyValue, String> {
    match source.split_once('=') {
        None => Err("Each element must be in the format: 'Key=Value'".to_string()),
        Some((key, value)) => Ok(KeyValue::new(key.to_string(), value.to_string())),
    }
}

/// Get Default Processor Tags
/// ## Example:
/// let _ = default_tracing_tags(git_version!(args = ["--abbrev=12", "--always"]),
/// env!("CARGO_PKG_VERSION"));
pub fn default_tracing_tags(git_commit: &str, cargo_version: &str) -> Vec<KeyValue> {
    vec![
        KeyValue::new("git.commit", git_commit.to_string()),
        KeyValue::new("crate.version", cargo_version.to_string()),
    ]
}

/// Name of the OTEL_BSP_MAX_EXPORT_BATCH_SIZE variable
pub const OTEL_BSP_MAX_EXPORT_BATCH_SIZE_NAME: &str = "OTEL_BSP_MAX_EXPORT_BATCH_SIZE";
/// The value of OTEL_BSP_MAX_EXPORT_BATCH_SIZE to be used with JAEGER
pub const OTEL_BSP_MAX_EXPORT_BATCH_SIZE_JAEGER: &str = "64";
/// Set the OTEL variables for a jaeger configuration
pub fn set_jaeger_env() {
    // if not set, default it to our jaeger value
    if std::env::var(OTEL_BSP_MAX_EXPORT_BATCH_SIZE_NAME).is_err() {
        std::env::set_var(
            OTEL_BSP_MAX_EXPORT_BATCH_SIZE_NAME,
            OTEL_BSP_MAX_EXPORT_BATCH_SIZE_JAEGER,
        );
    }
}

/// Mix the `RUST_LOG` `EnvFilter` with `RUST_LOG_SILENCE`.
/// This is useful when we want to bulk-silence certain crates by default.
pub fn rust_log_add_quiet_defaults(
    current: tracing_subscriber::EnvFilter,
) -> tracing_subscriber::EnvFilter {
    let rust_log_silence = std::env::var("RUST_LOG_SILENCE");
    let silence = match &rust_log_silence {
        Ok(quiets) => quiets.as_str(),
        Err(_) => super::constants::RUST_LOG_SILENCE_DEFAULTS,
    };

    tracing_subscriber::EnvFilter::try_new(match silence.is_empty() {
        true => current.to_string(),
        false => format!("{current},{silence}"),
    })
    .unwrap()
}

/// Initialise tracing and optionally opentelemetry.
/// Tracing will have a stdout subscriber with pretty formatting.
pub fn init_tracing(
    service_name: &str,
    tracing_tags: Vec<KeyValue>,
    jaeger: Option<String>,
    nats: TypedNats,
) {
    init_tracing_ext(service_name, tracing_tags, jaeger, FmtLayer::Stdout, nats);
}

/// Fmt Layer for console output.
pub enum FmtLayer {
    /// Output traces to stdout.
    Stdout,
    /// Output traces to stderr.
    Stderr,
    /// Don't output traces to console.
    None,
}

/// Initialise tracing and optionally opentelemetry.
/// Tracing will have a stdout subscriber with pretty formatting.
pub fn init_tracing_ext<T: std::net::ToSocketAddrs>(
    service_name: &str,
    mut tracing_tags: Vec<KeyValue>,
    jaeger: Option<T>,
    fmt_layer: FmtLayer,
    nats: TypedNats,
) {
    let filter = rust_log_add_quiet_defaults(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
    );

    let (stdout, stderr) = match fmt_layer {
        FmtLayer::Stdout => (Some(tracing_subscriber::fmt::layer().pretty()), None),
        FmtLayer::Stderr => (
            None,
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .pretty(),
            ),
        ),
        FmtLayer::None => (None, None),
    };

    let subscriber = Registry::default().with(filter).with(stdout).with(stderr);

    let mut nats_tracing_handle = EventHandle::init().unwrap();
    let filter = filter::Targets::new().with_target("nats", Level::INFO);
    nats_tracing_handle.attach_nats(nats).unwrap();
    //subscriber.with(nats_tracing_handle.layer.with_filter(filter));

    match jaeger {
        Some(jaeger) => {
            tracing_tags.append(&mut default_tracing_tags(
                super::raw_version_str(),
                env!("CARGO_PKG_VERSION"),
            ));
            let tracing_tags =
                tracing_tags
                    .into_iter()
                    .fold(Vec::<KeyValue>::new(), |mut acc, kv| {
                        if !acc.iter().any(|acc| acc.key == kv.key) {
                            acc.push(kv);
                        }
                        acc
                    });
            set_jaeger_env();

            global::set_text_map_propagator(TraceContextPropagator::new());
            let tracer = opentelemetry_jaeger::new_agent_pipeline()
                .with_endpoint(jaeger)
                .with_service_name(service_name)
                .with_trace_config(
                    opentelemetry::sdk::trace::Config::default()
                        .with_resource(Resource::new(tracing_tags)),
                )
                .install_batch(opentelemetry::runtime::TokioCurrentThread)
                .expect("Should be able to initialise the exporter");
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            subscriber
                .with(nats_tracing_handle.layer.with_filter(filter))
                .with(telemetry)
                .init();
        }
        None => subscriber
            .with(nats_tracing_handle.layer.with_filter(filter))
            .init(),
    };
}

/// Flush the traces from the tracer provider.
/// todo: force flush the traces.
pub fn flush_traces() {
    opentelemetry::global::shutdown_tracer_provider();
}
