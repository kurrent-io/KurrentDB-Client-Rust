#![allow(unused_attributes)]
#![allow(unused_imports)]
#![allow(unused_results)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use std::error::Error;
use kurrentdb::Client;

type Result<A> = std::result::Result<A, Box<dyn Error>>;

pub async fn user_certificates() -> Result<()> {
    // region client-with-user-certificates
    let settings = "kurrentdb://admin:changeit@{endpoint}?tls=true&userCertFile={pathToCaFile}&userKeyFile={pathToKeyFile}".parse()?;
    let client = Client::new(settings)?;
    // endregion client-with-user-certificates

    Ok(())
}
