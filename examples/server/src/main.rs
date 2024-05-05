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
        .with_endpoint("http://localhost:4317");

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
        .arg(arg!(-p --port <PORT> "Port used for management API").required(true))
        .arg(
            arg!(--peer <PEER_ADDRESS> "The address of a peer node")
                .action(ArgAction::Append)
                .required(true),
        )
        .get_matches();

    let port = matches.get_one::<String>("port").unwrap();
    let addr = SocketAddr::from_str(&format!("127.0.0.1:{}", port))
        .expect("Failed to construct SocketAddr");
    let peers: Vec<_> = matches.get_many::<String>("peer").unwrap().collect();
    println!("Peers {:?}", peers);

    let id = format!("railyard-{}", port);
    setup_tracing(id.clone());

    let router = Railyard::new_server(id, peers).await?;

    router.serve(addr).await?;

    Ok(())
}
