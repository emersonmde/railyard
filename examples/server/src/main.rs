use std::net::SocketAddr;
use std::str::FromStr;

use clap::{arg, command, ArgAction};
use railyard::Railyard;

use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Registry};

fn setup_tracing(id: String) {
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_timeout(Duration::from_secs(3))
        .with_endpoint(
            std::env::var("OTLP_ENDPOINT").unwrap_or("http://localhost:4317".to_string()),
        );

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
            opentelemetry_sdk::Resource::new(vec![
                opentelemetry::KeyValue::new("service.name", "railyard"),
                opentelemetry::KeyValue::new("server.id", id),
            ]),
        ))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to install OpenTelemetry pipeline");

    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    // Set up the format subscriber for local logging
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(true)
        .with_writer(std::io::stdout);

    // Combine both layers in a `Registry`
    let subscriber = Registry::default()
        .with(telemetry_layer)
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env());

    // Set the combined subscriber as the global default
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uuid::new_v4().to_string()

    let matches = command!()
        .arg(arg!(-p --port <PORT> "Port used for management API"))
        .arg(arg!(--peer <PEER_ADDRESS> "The address of a peer node").action(ArgAction::Append))
        .get_matches();

    // TODO: Change Dockerfile to use arguments instead of environment variables
    let port = if let Some(port) = matches.get_one::<String>("port") {
        port.to_string()
    } else {
        std::env::var("PORT")
            .expect("`--port <PORT>` argument or PORT environment variable must be set")
    };
    let addr =
        SocketAddr::from_str(&format!("0.0.0.0:{}", port)).expect("Failed to construct SocketAddr");
    let peers: Vec<String> = if let Some(peers_cli) = matches.get_many::<String>("peer") {
        peers_cli.map(|s| s.to_string()).collect()
    } else {
        std::env::var("PEERS")
            .expect("`--peer <PEER_ADDRESS>` argument or PEERS environment variable must be set")
            .split(',')
            .map(|s| s.to_string())
            .collect()
    };
    let server_id = std::env::var("SERVER_ID").unwrap_or_else(|_| format!("railyard-{}", port));
    setup_tracing(server_id.clone());
    tracing::info!(
        "{}: Starting server on port {} with peers {:?}",
        server_id,
        port,
        peers
    );

    let router = Railyard::new_server(server_id, peers).await?;

    router.serve(addr).await?;

    Ok(())
}
