pub async fn tests(port: u16) -> eyre::Result<()> {
    let user_cert = "certs/user-admin/user-admin.crt";
    let user_key = "certs/user-admin/user-admin.key";
    let setts = format!(
        "esdb://localhost:{}?tlsVerifyCert=false&usercertfile={}&userkeyfile={}",
        port, user_cert, user_key
    )
    .parse()?;
    let client = eventstore::Client::new(setts)?;

    let mut streams = client.read_all(&Default::default()).await?;

    streams.next().await?;

    Ok(())
}
