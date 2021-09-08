mod encode;
mod json_string;

pub use encode::{Json, Object, JsonString};

#[macro_export]
macro_rules! jsons {
    ($e:tt) => { json!($e).to_string() }
}

#[macro_export]
macro_rules! json {
    (null) => { json::Json::Null };

    ([$($e:tt),*]) => {
        crate::json::Json::Array(vec![
            $(
            json!($e),
            )*
        ])
    };

    ([$($e:tt,)*]) => { json!([$($e),*]) };

    ({$($name:ident: $e:tt),*}) => {{
        let mut map = crate::json::Object::new();

        $(
        map.insert(stringify!($name), json!($e));
        )*

        crate::json::Json::Object(map)
    }};

    ({$($name:ident: $e:tt,)*}) => { json!({$($name: $e),*}) };

    ($e:expr) => { crate::json::Jsonable::into_json($e) };
}

pub trait Jsonable {
    fn into_json(self) -> Json;
}

impl Jsonable for Json {
    fn into_json(self) -> Json { self }
}

impl Jsonable for bool {
    fn into_json(self) -> Json { Json::Boolean(self) }
}

impl Jsonable for &str {
    fn into_json(self) -> Json { Json::String(self.into()) }
}

impl Jsonable for String {
    fn into_json(self) -> Json { Json::String(self.into()) }
}

impl Jsonable for f64 {
    fn into_json(self) -> Json { Json::Number(self) }
}

impl Jsonable for i32 {
    fn into_json(self) -> Json { Json::Number(self as f64) }
}

impl Jsonable for u8 {
    fn into_json(self) -> Json { Json::Number(self as f64) }
}

impl Jsonable for usize {
    fn into_json(self) -> Json { Json::Number(self as f64) }
}