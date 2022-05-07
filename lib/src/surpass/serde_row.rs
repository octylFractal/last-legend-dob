use std::io::Cursor;

use serde::de::{DeserializeOwned, DeserializeSeed, Deserializer, Error, SeqAccess, Visitor};

use crate::error::LastLegendError;
use crate::surpass::sheet_info::{Column, DataValue};

pub fn from_row<T: DeserializeOwned>(
    columns: &[Column],
    fixed_row_size: u64,
    row: Vec<u8>,
) -> Result<T, LastLegendError> {
    let mut deserializer = SerdeRowReader {
        columns,
        fixed_row_size,
        row,
        col_index: 0,
    };
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.col_index == columns.len() {
        Ok(t)
    } else {
        Err(LastLegendError::custom(format!(
            "Did not consume all columns, {}/{}",
            deserializer.col_index,
            columns.len()
        )))
    }
}

/// Reads a row as [serde::Deserialize] types.
struct SerdeRowReader<'col> {
    columns: &'col [Column],
    fixed_row_size: u64,
    row: Vec<u8>,
    col_index: usize,
}

impl<'a, 'col, 'de> SeqAccess<'de> for &'a mut SerdeRowReader<'col> {
    type Error = LastLegendError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.columns.get(self.col_index) {
            Some(_) => seed.deserialize(&mut **self).map(Some),
            None => Ok(None),
        }
    }
}

impl<'a, 'si, 'de> Deserializer<'de> for &'a mut SerdeRowReader<'si> {
    type Error = LastLegendError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let column = match self.columns.get(self.col_index) {
            Some(c) => c,
            None => return Err(LastLegendError::custom("No more columns available")),
        };
        self.col_index += 1;
        match column.read_value(Cursor::new(&mut self.row), self.fixed_row_size)? {
            DataValue::String(s) => visitor.visit_string(s),
            DataValue::Bool(b) => visitor.visit_bool(b),
            DataValue::I8(v) => visitor.visit_i8(v),
            DataValue::U8(v) => visitor.visit_u8(v),
            DataValue::I16(v) => visitor.visit_i16(v),
            DataValue::U16(v) => visitor.visit_u16(v),
            DataValue::I32(v) => visitor.visit_i32(v),
            DataValue::U32(v) => visitor.visit_u32(v),
            DataValue::F32(v) => visitor.visit_f32(v),
            DataValue::I64(v) => visitor.visit_i64(v),
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option enum identifier ignored_any
    }
}
