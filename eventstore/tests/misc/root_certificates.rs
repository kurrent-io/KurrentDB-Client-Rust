pub async fn tests(port: u16) -> eyre::Result<()> {
    let root_cert = "certs/ca/ca.crt";

    let setts = format!(
        "esdb://admin:changeit@localhost:{}?tlsVerifyCert=true&tls=true&tlsCaFile={}",
        port,
        root_cert
    )
    .parse()?;
    let client = eventstore::Client::new(setts)?;

    let mut streams = client.read_all(&Default::default()).await?;

    streams.next().await?;

    Ok(())
}
