use std::sync::Arc;

use neon::prelude::*;
use tokio::runtime::Runtime;

mod client;

lazy_static::lazy_static! {
    static ref RUNTIME: Arc<Runtime> = {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        Arc::new(runtime)
    };
}

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}



#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("hello", hello)?;
    cx.export_function("rustCreateClient", client::create)?;

    Ok(())
}
