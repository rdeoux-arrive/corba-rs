use super::error::{Error, Result};
use log::trace;
use serde::{ser, Serialize};
use std::io::{self, Write};

pub struct Serializer<W>
where
    W: Write,
{
    stream: W,
    le: bool,
    align: usize,
    indent: String,
}

macro_rules! from_bits {
    ($v:ident as $t:ident) => { $t::from_le_bytes($v.to_le_bytes())};
}

pub fn to_bytes<T>(le: bool, value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut bytes = Vec::new();
    to_writer(&mut bytes, le, value)?;
    Ok(bytes)
}

pub fn to_writer<W, T>(writer: &mut W, le: bool, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    value.serialize(&mut Serializer::new(writer).with_le(le))
}

impl<W> Serializer<W>
where
    W: Write,
{
    pub fn new(stream: W) -> Self {
        Self {
            stream,
            le: false,
            align: 0,
            indent: String::new(),
        }
    }

    pub fn with_le(self, le: bool) -> Self {
        Self {
            stream: self.stream,
            le,
            align: self.align,
            indent: self.indent,
        }
    }

    pub fn writer(&self) -> &W {
        &self.stream
    }

    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.stream
    }

    fn write_u8(&mut self, v: u8) -> io::Result<()> {
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        })
    }

    fn write_u16(&mut self, v: u16) -> io::Result<()> {
        if self.align & 1 != 0 {
            self.write_all(&[0])?
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        })
    }

    fn write_u32(&mut self, v: u32) -> io::Result<()> {
        if self.align & 3 != 0 {
            self.write_all(&[0; 3][..(1 + !self.align) & 3])?
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        })
    }

    fn write_u64(&mut self, v: u64) -> io::Result<()> {
        if self.align & 7 != 0 {
            self.write_all(&[0; 7][..(1 + !self.align) & 7])?
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        })
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.stream.write_all(buf)?;
        self.align += buf.len();
        Ok(())
    }

    fn push_indent(&mut self) {
        self.indent.push_str("  ")
    }

    fn pop_indent(&mut self) {
        self.indent.truncate(self.indent.len() - 2)
    }
}

impl<'a, W> ser::Serializer for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, W>;
    type SerializeTuple = Self;
    type SerializeTupleStruct = SerializeTupleStruct<'a, W>;
    type SerializeTupleVariant = SerializeTupleVariant<'a, W>;
    type SerializeMap = Self;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = SerializeStructVariant<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        trace!("{}serialize {}", self.indent, v);
        self.write_u8(if v { 1 } else { 0 }).map_err(Error::Io)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        trace!("{}serialize i8 {} ({1:#04x})", self.indent, v);
        self.write_u8(from_bits!(v as u8)).map_err(Error::Io)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        trace!("{}serialize i16 {} ({1:#06x})", self.indent, v);
        self.write_u16(from_bits!(v as u16)).map_err(Error::Io)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        trace!("{}serialize i32 {} ({1:#010x})", self.indent, v);
        self.write_u32(from_bits!(v as u32)).map_err(Error::Io)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        trace!("{}serialize i64 {} ({1:#018x})", self.indent, v);
        self.write_u64(from_bits!(v as u64)).map_err(Error::Io)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        trace!("{}serialize u8 {} ({1:#04x})", self.indent, v);
        self.write_u8(v).map_err(Error::Io)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        trace!("{}serialize u16 {} ({1:#06x})", self.indent, v);
        self.write_u16(v).map_err(Error::Io)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        trace!("{}serialize u32 {} ({1:#010x})", self.indent, v);
        self.write_u32(v).map_err(Error::Io)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        trace!("{}serialize u64 {} ({1:#018x})", self.indent, v);
        self.write_u64(v).map_err(Error::Io)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        trace!("{}serialize f32 {} ({:#010x})", self.indent, v, v.to_bits());
        self.write_u32(v.to_bits()).map_err(Error::Io)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        trace!("{}serialize f64 {} ({:#018x})", self.indent, v, v.to_bits());
        self.write_u64(v.to_bits()).map_err(Error::Io)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        trace!("{}serialize char {:?}", self.indent, v);
        self.write_u8(v as u8).map_err(Error::Io)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        trace!("{}serialize str {:?}", self.indent, v);
        self.write_u32(v.len() as u32 + 1)?;
        self.write_all(v.as_bytes())?;
        self.write_u8(0).map_err(Error::Io)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        trace!(
            "{}serialize bytes ({} byte{})",
            self.indent,
            v.len(),
            if v.len() < 2 { "s" } else { "" }
        );
        self.write_u32(v.len() as u32)?;
        self.write_all(v).map_err(Error::Io)
    }

    fn serialize_none(self) -> Result<()> {
        trace!("{}serialize None", self.indent);
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize Some(..)...", self.indent);
        self.push_indent();
        value.serialize(&mut *self)?;
        self.pop_indent();
        trace!("{}serialize Some(..)... done", self.indent);
        Ok(())
    }
    fn serialize_unit(self) -> Result<()> {
        trace!("{}serialize ()", self.indent);
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        trace!("{}serialize {}", self.indent, name);
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        trace!("{}serialize {}::{}", self.indent, name, variant);
        self.write_u32(variant_index).map_err(Error::Io)
    }
    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize {}...", self.indent, name);
        self.push_indent();
        value.serialize(&mut *self)?;
        self.pop_indent();
        trace!("{}serialize {}... done", self.indent, name);
        Ok(())
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index)?;
        self.push_indent();
        value.serialize(&mut *self)?;
        self.pop_indent();
        trace!("{}serialize {}::{}... done", self.indent, name, variant);
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        trace!("{}serialize seq...", self.indent);
        if let Some(len) = len {
            self.write_u32(len as u32)?
        }
        self.push_indent();
        Ok(SerializeSeq { ser: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        trace!("{}serialize tuple...", self.indent);
        self.push_indent();
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        trace!(
            "{}serialize tuple struct {} ({} fields)...",
            self.indent,
            name,
            len
        );
        self.push_indent();
        Ok(Self::SerializeTupleStruct { ser: self, name })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index)?;
        self.push_indent();
        Ok(Self::SerializeTupleVariant {
            ser: self,
            name,
            variant,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        trace!("{}serialize map...", self.indent);
        if let Some(len) = len {
            self.write_u32(len as u32)?;
        }
        self.push_indent();
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        trace!("{}serialize struct {}...", self.indent, name);
        self.push_indent();
        Ok(Self::SerializeStruct { ser: self, name })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index)?;
        self.push_indent();
        Ok(Self::SerializeStructVariant {
            ser: self,
            name,
            variant,
        })
    }
}

pub struct SerializeSeq<'a, W>
where
    W: Write,
{
    ser: &'a mut Serializer<W>,
}

impl<'a, W> ser::SerializeSeq for SerializeSeq<'a, W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.pop_indent();
        trace!("{}serialize seq... done", self.ser.indent);
        Ok(())
    }
}

impl<'a, W> ser::SerializeTuple for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.pop_indent();
        trace!("{}serialize tuple... done", self.indent);
        Ok(())
    }
}

pub struct SerializeTupleStruct<'a, W>
where
    W: Write,
{
    ser: &'a mut Serializer<W>,
    name: &'static str,
}

impl<'a, W> ser::SerializeTupleStruct for SerializeTupleStruct<'a, W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize field {}...", self.ser.indent, self.name);
        self.ser.push_indent();
        let v = value.serialize(&mut *self.ser);
        self.ser.pop_indent();
        trace!("{}serialize field {}... done", self.ser.indent, self.name);
        v
    }

    fn end(self) -> Result<()> {
        let (ser, name) = (self.ser, self.name);
        ser.pop_indent();
        trace!("{}serialize field {}... done", ser.indent, name);
        Ok(())
    }
}

pub struct SerializeTupleVariant<'a, W>
where
    W: Write,
{
    ser: &'a mut Serializer<W>,
    name: &'static str,
    variant: &'static str,
}

impl<'a, W> ser::SerializeTupleVariant for SerializeTupleVariant<'a, W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        let (ser, name, variant) = (self.ser, self.name, self.variant);
        ser.pop_indent();
        trace!("{}serialize {}::{}... done", ser.indent, name, variant);
        Ok(())
    }
}

impl<'a, W> ser::SerializeMap for &'a mut Serializer<W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize key...", self.indent);
        self.push_indent();
        let v = key.serialize(&mut **self);
        self.pop_indent();
        trace!("{}serialize key... done", self.indent);
        v
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!("{}serialize value...", self.indent);
        self.push_indent();
        let v = value.serialize(&mut **self);
        self.pop_indent();
        trace!("{}serialize value... done", self.indent);
        v
    }

    fn end(self) -> Result<()> {
        self.pop_indent();
        trace!("{}serialize map... done", self.indent);
        Ok(())
    }
}

pub struct SerializeStruct<'a, W>
where
    W: Write,
{
    ser: &'a mut Serializer<W>,
    name: &'static str,
}

impl<'a, W> ser::SerializeStruct for SerializeStruct<'a, W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!(
            "{}serialize field {}::{}...",
            self.ser.indent,
            self.name,
            key
        );
        self.ser.push_indent();
        value.serialize(&mut *self.ser)?;
        self.ser.pop_indent();
        trace!(
            "{}serialize field {}::{}... done",
            self.ser.indent,
            self.name,
            key
        );
        Ok(())
    }

    fn end(self) -> Result<()> {
        let (ser, name) = (self.ser, self.name);
        ser.pop_indent();
        trace!("{}serialize struct {}... done", ser.indent, name);
        Ok(())
    }
}

pub struct SerializeStructVariant<'a, W>
where
    W: Write,
{
    ser: &'a mut Serializer<W>,
    name: &'static str,
    variant: &'static str,
}

impl<'a, W> ser::SerializeStructVariant for SerializeStructVariant<'a, W>
where
    W: Write,
{
    type Ok = <&'a mut Serializer<W> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<W> as ser::Serializer>::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        trace!(
            "{}serialize {}::{}::{}...",
            self.ser.indent,
            self.name,
            self.variant,
            key
        );
        let v = value.serialize(&mut *self.ser);
        trace!(
            "{}serialize {}::{}::{}... done",
            self.ser.indent,
            self.name,
            self.variant,
            key
        );
        v
    }

    fn end(self) -> Result<()> {
        let (ser, name, variant) = (self.ser, self.name, self.variant);
        ser.pop_indent();
        trace!("{}serialize {}::{}... done", ser.indent, name, variant);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_primitive() {
        assert_eq!(to_bytes(false, &false).unwrap(), vec![0x00]);
        assert_eq!(to_bytes(true, &false).unwrap(), vec![0x00]);
        assert_eq!(to_bytes(false, &true).unwrap(), vec![0x01]);
        assert_eq!(to_bytes(true, &true).unwrap(), vec![0x01]);

        assert_eq!(to_bytes(false, &1_i8).unwrap(), vec![0x01]);
        assert_eq!(to_bytes(true, &1_i8).unwrap(), vec![0x01]);
        assert_eq!(to_bytes(false, &-2_i8).unwrap(), vec![0xfe]);
        assert_eq!(to_bytes(true, &-2_i8).unwrap(), vec![0xfe]);

        assert_eq!(to_bytes(false, &1_i16).unwrap(), vec![0x00, 0x01]);
        assert_eq!(to_bytes(true, &1_i16).unwrap(), vec![0x01, 0x00]);
        assert_eq!(to_bytes(false, &-2_i16).unwrap(), vec![0xff, 0xfe]);
        assert_eq!(to_bytes(true, &-2_i16).unwrap(), vec![0xfe, 0xff]);

        assert_eq!(
            to_bytes(false, &1_i32).unwrap(),
            vec![0x00, 0x00, 0x00, 0x01]
        );
        assert_eq!(
            to_bytes(true, &1_i32).unwrap(),
            vec![0x01, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            to_bytes(false, &-2_i32).unwrap(),
            vec![0xff, 0xff, 0xff, 0xfe]
        );
        assert_eq!(
            to_bytes(true, &-2_i32).unwrap(),
            vec![0xfe, 0xff, 0xff, 0xff]
        );

        assert_eq!(
            to_bytes(false, &1_i64).unwrap(),
            vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        );
        assert_eq!(
            to_bytes(true, &1_i64).unwrap(),
            vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            to_bytes(false, &-2_i64).unwrap(),
            vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe]
        );
        assert_eq!(
            to_bytes(true, &-2_i64).unwrap(),
            vec![0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn serialize_unaligned() {
        #[derive(Serialize)]
        struct Unaligned {
            a: u8,
            b: u16,
            c: u8,
            d: u32,
        }

        let unaligned = Unaligned {
            a: 0x11,
            b: 0x2233,
            c: 0x44,
            d: 0x5566_7788,
        };
        assert_eq!(
            to_bytes(false, &unaligned).unwrap(),
            vec![0x11, 0x00, 0x22, 0x33, 0x44, 0x00, 0x00, 0x00, 0x55, 0x66, 0x77, 0x88]
        )
    }
}
