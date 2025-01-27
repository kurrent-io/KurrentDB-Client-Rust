use eventstore::{Client, ClientSettings, ReadStreamOptions, StreamPosition};
use neon::{object::Object, prelude::{Context, FunctionContext}, result::JsResult, types::{JsBigInt, JsFunction, JsObject, JsString}};

use crate::RUNTIME;

pub fn create(mut cx: FunctionContext) -> JsResult<JsObject> {
    let conn_string = cx.argument::<JsString>(0)?.value(&mut cx);

    let setts = match conn_string.parse::<ClientSettings>() {
        Err(e) => cx.throw_error(e.to_string())?,
        Ok(s) => s,
    };
 
    let client = match Client::with_runtime_handle(RUNTIME.handle().clone(), setts) {
        Err(e) => cx.throw_error(e.to_string())?,
        Ok(c) => c,
    };

    let obj = cx.empty_object();

    let local_client = client.clone();
    let client_read_stream = JsFunction::new(&mut cx, move |cx| read_stream(local_client.clone(), cx))?;

    obj.set(&mut cx, "readStream", client_read_stream)?;

    Ok(obj)
}

pub fn read_stream(client: Client, mut cx: FunctionContext) -> JsResult<JsObject> {
    let stream_name = cx.argument::<JsString>(0)?.value(&mut cx);
    let params = cx.argument::<JsObject>(1)?;
    let options = ReadStreamOptions::default();

    let direction_str = params.get::<JsString, _,_>(&mut cx, "direction")?.value(&mut cx);
    let options = match direction_str.as_str() {
        "forward" => options.forwards(),
        "backward" => options.backwards(),
        x => cx.throw_error(format!("invalid direction value: '{}'", x))?,
    };

    let options = match params.get::<JsBigInt, _, _>(&mut cx, "fromRevision")?.to_u64(&mut cx) {
        Ok(r) => if r == 0 {
            options.position(StreamPosition::Start)
        } else {
            options.position(StreamPosition::Position(r))
        }
        Err(e) => cx.throw_error(e.to_string())?, 
    };

    let options = match params.get::<JsBigInt, _, _>(&mut cx, "maxCount")?.to_u64(&mut cx) {
        Ok(r) => options.max_count(r as usize),
        Err(e) => cx.throw_error(e.to_string())?, 
    };

    client.read_stream(stream_name.as_str(), &options);


    todo!()
}
