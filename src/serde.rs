//! Partial Serialize/Deserialize implementation for Zval to support JSON/form un/marshalling.
//! `ZvalSerializer` is a wrapper to serialize `Zval` references.
use std::fmt::Formatter;

use ext_php_rs::types::{ZendHashTable, Zval};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub struct ZvalSerializer<'a>(pub &'a Zval);

impl<'a> Serialize for ZvalSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use ext_php_rs::flags::DataType::*;

        match self.0.get_type() {
            Null => serializer.serialize_none(),
            False => serializer.serialize_bool(false),
            True => serializer.serialize_bool(true),
            Bool => serializer.serialize_bool(self.0.bool().unwrap()),
            Long => serializer.serialize_i64(self.0.long().unwrap()),
            Double => serializer.serialize_f64(self.0.double().unwrap()),
            String => serializer.serialize_str(self.0.string().unwrap().as_str()),
            Array => {
                let array = self.0.array().unwrap();
                if array.has_sequential_keys() {
                    let mut seq = serializer.serialize_seq(Some(array.len()))?;
                    for value in array.values() {
                        seq.serialize_element(&ZvalSerializer(value))?;
                    }
                    seq.end()
                } else {
                    let mut map = serializer.serialize_map(Some(array.len()))?;
                    for (idx, key, value) in array.iter() {
                        if let Some(key) = key {
                            map.serialize_entry(&key, &ZvalSerializer(value))?;
                        } else {
                            map.serialize_entry(&idx, &ZvalSerializer(value))?;
                        }
                    }
                    map.end()
                }
            }
            _ => unimplemented!(),
        }
    }
}

pub struct ZvalDeserializer(pub Zval);

impl<'de> Deserialize<'de> for ZvalDeserializer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SerdeZvalVisitor {})
    }
}

struct SerdeZvalVisitor {}

impl<'de> Visitor<'de> for SerdeZvalVisitor {
    type Value = ZvalDeserializer;

    fn expecting(&self, _formatter: &mut Formatter) -> std::fmt::Result {
        todo!()
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_bool(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_long(v as i64);
        Ok(ZvalDeserializer(val))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_double(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_double(v);
        Ok(ZvalDeserializer(val))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_null();
        Ok(ZvalDeserializer(val))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_null();
        Ok(ZvalDeserializer(val))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut val = Zval::new();
        val.set_string(v, false).unwrap();
        Ok(ZvalDeserializer(val))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut val = Zval::new();
        let mut array = Vec::new();
        while let Some(el) = seq.next_element::<ZvalDeserializer>()? {
            array.push(el.0);
        }
        val.set_array(array).unwrap();
        Ok(ZvalDeserializer(val))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut val = Zval::new();
        let mut hashtable = ZendHashTable::new();
        while let Some((key, value)) = map.next_entry::<String, ZvalDeserializer>()? {
            hashtable.insert(&key, value.0).unwrap();
        }
        val.set_hashtable(hashtable);
        Ok(ZvalDeserializer(val))
    }
}
