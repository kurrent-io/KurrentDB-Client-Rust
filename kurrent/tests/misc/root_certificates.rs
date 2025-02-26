use tracing::debug;

async fn test_with_valid_root_certificate(port: u16) -> eyre::Result<()> {
    let root_cert = "certs/ca/ca.crt";

    let setts = format!(
        "esdb://admin:changeit@localhost:{}?tlsVerifyCert=true&tls=true&tlsCaFile={}",
        port, root_cert
    )
    .parse()?;
    let client = kurrent::Client::new(setts)?;

    let mut streams = client.read_all(&Default::default()).await?;

    streams.next().await?;

    Ok(())
}

async fn test_with_invalid_certificate(port: u16) -> eyre::Result<()> {
    // invalid root certificate
    let root_cert = "certs/node1/node.crt";

    let setts = format!(
        "esdb://admin:changeit@localhost:{}?tlsVerifyCert=true&tls=true&tlsCaFile={}",
        port, root_cert
    )
    .parse()?;
    let client = kurrent::Client::new(setts)?;

    let result = client.read_all(&Default::default()).await;

    assert!(
        result.is_err(),
        "Expected an error due to invalid certificate"
    );

    Ok(())
}

pub async fn tests(port: u16) -> eyre::Result<()> {
    debug!("Before test_with_valid_root_certificate…");
    test_with_valid_root_certificate(port).await?;
    debug!("Complete");

    debug!("Before test_with_invalid_certificate…");
    test_with_invalid_certificate(port).await?;
    debug!("Complete");

    Ok(())
}
