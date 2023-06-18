//! This example demonstrates how to make multiple outgoing connections on a single UDP socket.
//!
//! Checkout the `README.md` for guidance.

use k8s_openapi::api::core::v1::Secret;
use kube::{Api, Client};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{extension::SubjectAlternativeName, X509NameBuilder, X509Req, X509},
};
use quinn::{ClientConfig, Endpoint, ServerConfig};
use std::{error::Error, net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr1 = "127.0.0.1:8000".parse().unwrap();
    // let addr2 = "127.0.0.1:5001".parse().unwrap();
    // let addr3 = "127.0.0.1:5002".parse().unwrap();
    run_server(addr1).await?;
    // let server2_cert = run_server(addr2)?;
    // let server3_cert = run_server(addr3)?;

    let client = make_client_endpoint(
        "127.0.0.1:0".parse().unwrap(),
        // &[&server1_cert, &server2_cert, &server3_cert],
    )
    .await?;

    // connect to multiple endpoints using the same socket/endpoint
    tokio::join!(
        run_client(&client, addr1),
        // run_client(&client, addr2),
        // run_client(&client, addr3),
    );

    // Make sure the server has a chance to clean up
    client.wait_idle().await;

    Ok(())
}

/// Runs a QUIC server bound to given address and returns server certificate.
async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let endpoint = make_server_endpoint(addr).await?;
    // accept a single connection
    tokio::spawn(async move {
        let connection = endpoint.accept().await.unwrap().await.unwrap();
        println!(
            "[server] incoming connection: addr={}",
            connection.remote_address()
        );
    });

    Ok(())
}

/// Attempt QUIC connection with the given server address.
async fn run_client(endpoint: &Endpoint, server_addr: SocketAddr) {
    let connect = endpoint.connect(server_addr, "localhost").unwrap();
    let connection = connect.await;
    match connection {
        Ok(connection) => {
            println!("[client] connected: addr={}", connection.remote_address());
        }
        Err(e) => {
            println!("[client] connection failed: {:?}", e);
        }
    }
}

/// Constructs a QUIC endpoint configured for use a client only.
///
/// ## Args
///
/// - server_certs: list of trusted certificates.
#[allow(unused)]
pub async fn make_client_endpoint(
    bind_addr: SocketAddr,
) -> Result<Endpoint, Box<dyn Error>> {
    let client_cfg = configure_client().await?;
    let mut endpoint = Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(client_cfg);
    Ok(endpoint)
}

/// Constructs a QUIC endpoint configured to listen for incoming connections on a certain address
/// and port.
///
/// ## Returns
///
/// - a stream of incoming QUIC connections
/// - server certificate serialized into DER format
#[allow(unused)]
pub async fn make_server_endpoint(
    bind_addr: SocketAddr,
) -> Result<(Endpoint), Box<dyn Error>> {
    let server_config = configure_server().await?;
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok(endpoint)
}

/// Builds default quinn client config and trusts given certificates.
///
/// ## Args
///
/// - server_certs: a list of trusted certificates in DER format.
async fn configure_client() -> Result<ClientConfig, Box<dyn Error>> {
    let mut certs = rustls::RootCertStore::empty();

    let (ca_cert, _) = get_ca_cert_key().await?;
    let ca_cert = rustls::Certificate(ca_cert.to_der()?);
    certs.add(&ca_cert)?;

    let client_config = ClientConfig::with_root_certificates(certs);
    Ok(client_config)
}

async fn get_ca_cert_key_from_k8s() -> Result<(X509, PKey<Private>), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());
    let secrets: Api<Secret> = Api::namespaced(client, &namespace);

    let secret_name = "ca-secret"; // name of the secret
    let secret = secrets.get(secret_name).await?;

    let mut cert_bytes = vec![];
    let mut key_bytes = vec![];

    if let Some(data) = secret.data {
        if let Some(cert) = data.get("ca.crt") {
            cert_bytes = cert.0.clone();
        }

        if let Some(key) = data.get("ca.key") {
            key_bytes = key.0.clone();
        }
    }

    Ok((
        X509::from_pem(&cert_bytes)?,
        PKey::private_key_from_pem(&key_bytes)?,
    ))
}

fn get_ca_cert_key_from_disk() -> Result<(X509, PKey<Private>), Box<dyn std::error::Error>> {
    let ca_cert = std::fs::read("ca.crt");
    let ca_key = std::fs::read("ca.key");

    if let Err(e) = ca_cert {
        eprintln!("Failed to read ca.crt: {}", e);
        return Err(e.into());
    }

    if let Err(e) = ca_key {
        eprintln!("Failed to read ca.key: {}", e);
        return Err(e.into());
    }

    Ok((
        X509::from_pem(&ca_cert.unwrap())?,
        PKey::private_key_from_pem(&ca_key.unwrap())?,
    ))
}

async fn get_ca_cert_key() -> Result<(X509, PKey<Private>), Box<dyn std::error::Error>> {
    if std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
        && std::env::var("KUBERNETES_SERVICE_PORT").is_ok()
    {
        get_ca_cert_key_from_k8s().await
    } else {
        get_ca_cert_key_from_disk()
    }
}

/// Returns default server configuration along with its certificate.
async fn configure_server() -> Result<ServerConfig, Box<dyn Error>> {
    let (ca_cert, ca_key) = get_ca_cert_key().await?;
    println!("Imported CA Cert and Key");

    // Generate a new private key for the server
    // let server_key = openssl::pkey::PKey::generate_ed25519()?;
    let rsa = Rsa::generate(2048)?;
    let server_key = PKey::from_rsa(rsa)?;
    println!("Generated server key");

    // Generate a certificate signing request
    let mut csr = X509Req::builder()?;
    csr.set_pubkey(&server_key)?;

    let mut name_builder = X509NameBuilder::new()?;
    name_builder.append_entry_by_text("CN", "localhost")?;
    let name = name_builder.build();

    csr.set_subject_name(&name)?;
    csr.sign(&server_key, MessageDigest::sha256())?;
    let csr = csr.build();
    println!("Generated CSR");

    // Generate a new certificate and sign it with our CA
    let mut cert_builder = openssl::x509::X509::builder()?;
    cert_builder.set_version(2)?;
    cert_builder.set_pubkey(&server_key)?;
    let not_before = openssl::asn1::Asn1Time::days_from_now(0)?;
    let not_after = openssl::asn1::Asn1Time::days_from_now(365)?;
    cert_builder.set_not_before(&not_before)?;
    cert_builder.set_not_after(&not_after)?;
    cert_builder.set_issuer_name(ca_cert.subject_name())?;
    cert_builder.set_subject_name(csr.subject_name())?;

    // Add San
    let san_name = SubjectAlternativeName::new()
        .dns("localhost")
        .build(&cert_builder.x509v3_context(None, None))?;
    cert_builder.append_extension(san_name)?;

    cert_builder.sign(&ca_key, openssl::hash::MessageDigest::sha256())?;
    println!("Signed cert");

    let cert = cert_builder.build();
    println!("Build cert");

    let server_cert = cert.to_der()?;
    let priv_key = rustls::PrivateKey(server_key.private_key_to_der()?);
    let cert_chain = vec![rustls::Certificate(server_cert.clone())];

    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());

    Ok(server_config)
}
