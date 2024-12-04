use std::collections::HashMap;

use rmp_serde::encode::Error;
use serde::{
    ser::{self, Impossible, SerializeStruct},
    Serialize,
};

#[derive(Default)]
pub struct Serializer {
    map: HashMap<String, Vec<u8>>,
    stack: Vec<&'static str>,
}

impl Serializer {
    pub fn finish(self) -> HashMap<String, Vec<u8>> {
        self.map
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect bool")
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect i16")
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect i32")
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect i64")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect u64")
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect f32")
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect f64")
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect char")
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect str")
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect &[u8]")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect None")
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!("did not expect Some(T)")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect unit")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        panic!("did not expect unit variant")
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!("did not expect newtype struct")
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!("did not expect newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        panic!("did not expect sequence")
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        panic!("did not expect tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        panic!("did not expect tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        panic!("did not expect tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        panic!("did not expect map")
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(Compound { serializer: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        panic!("did not expect struct variant")
    }
}

pub struct Compound<'a> {
    serializer: &'a mut Serializer,
}

impl<'a> SerializeStruct for Compound<'a> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        if self.serializer.stack.is_empty() {
            self.serializer.stack.push(key);
            let output = value.serialize(&mut *self.serializer);
            self.serializer.stack.pop();
            return output;
        }

        let mut buffer = Vec::new();
        let mut rmp_serializer = rmp_serde::Serializer::new(&mut buffer).with_struct_map();
        value.serialize(&mut rmp_serializer)?;
        self.serializer.map.insert(
            format!("{}.{}", self.serializer.stack.join("."), key),
            buffer,
        );
        Ok(())
    }

    #[inline(always)]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
