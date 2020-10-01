pub fn process(v: &mut json::Value) -> std::io::Result<()> {
    match v {
        json::Value::Array(ref mut arr) => {
            arr.sort_by_cached_key(|v| json::to_string(&v)?);
            arr.iter_mut().for_each(|v| {
                process(v);
            })
        }
        json::Value::Object(ref mut obj) => {
            obj.iter_mut().for_each(|(k,v)| {
                process(v);
            })
        }
        _ => {}
    }
}

fn main() -> std::io::Result<()> {
    let mut val: json::Value = json::from_reader(std::io::stdin()).unwrap();
    process(&mut val)?;
    json::to_writer_pretty(std::io::stdout(), &val).unwrap();
    Ok(())
}

