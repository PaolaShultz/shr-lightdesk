//! Bounded C-LIGHT JSON/framing; no lighting calculations or endpoint startup.
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::{fmt, io::Read};

pub const MESSAGE_BYTES: usize = 65536;
pub const MAX_PAGES: usize = 16;
pub fn counter(v: &Value) -> Result<u64, String> {
    let s = v.as_str().ok_or("counter must be a decimal string")?;
    if s.is_empty() || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("noncanonical counter".into());
    }
    let n = s.parse::<u64>().map_err(|_| "counter overflow")?;
    if n == u64::MAX {
        return Err("counter exhausted".into());
    }
    Ok(n)
}
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded integer JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, x: bool) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_i64<E: de::Error>(self, x: i64) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_u64<E: de::Error>(self, x: u64) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_str<E: de::Error>(self, x: &str) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_string<E: de::Error>(self, x: String) -> Result<Strict, E> {
                Ok(Strict(x.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<Strict>()? {
                    v.push(x.0);
                }
                Ok(Strict(v.into()))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut v = Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if v.contains_key(&k) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    v.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(v.into()))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn json(bytes: &[u8], bound: usize) -> Result<Value, String> {
    if bytes.is_empty() || bytes.len() > bound {
        return Err("JSON byte capacity".into());
    }
    std::str::from_utf8(bytes).map_err(|_| "invalid UTF8")?;
    let (mut depth, mut quoted, mut escape) = (0usize, false, false);
    for &b in bytes {
        if quoted {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 12 {
                        return Err("JSON depth capacity".into());
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let x = Strict::deserialize(&mut d).map_err(|e| e.to_string())?;
    d.end().map_err(|e| e.to_string())?;
    Ok(x.0)
}
pub fn read_frame(r: &mut impl Read) -> Result<Option<Vec<u8>>, String> {
    let mut h = [0; 4];
    match r.read(&mut h[..1]).map_err(|e| e.to_string())? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!(),
    }
    r.read_exact(&mut h[1..])
        .map_err(|_| "incomplete frame header")?;
    let n = u32::from_be_bytes(h) as usize;
    if n == 0 || n > MESSAGE_BYTES {
        return Err("frame byte capacity".into());
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b)
        .map_err(|_| "incomplete frame payload")?;
    Ok(Some(b))
}
