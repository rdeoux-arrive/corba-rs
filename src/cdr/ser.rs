use super::error::Error;
use super::indent::Indent;
use alloc::vec::Vec;
use log::trace;
use serde::{ser, Serialize};

#[derive(Default)]
pub struct Serializer<T>
where
    T: Extend<u8>,
{
    stream: T,
    le: bool,
    align: usize,
    indent: Indent,
}

impl<T> Serializer<T>
where
    T: Extend<u8>,
{
    #[must_use]
    pub fn with_le(self, le: bool) -> Self {
        Self { le, ..self }
    }

    fn write_u8(&mut self, v: u8) {
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        });
    }

    fn write_u16(&mut self, v: u16) {
        if self.align & 1 != 0 {
            self.write_all(&[0]);
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        });
    }

    fn write_u32(&mut self, v: u32) {
        if self.align & 3 != 0 {
            self.write_all(&[0; 3][..(1 + !self.align) & 3]);
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        });
    }

    fn write_u64(&mut self, v: u64) {
        if self.align & 7 != 0 {
            self.write_all(&[0; 7][..(1 + !self.align) & 7]);
        }
        self.write_all(&if self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        });
    }

    fn write_all(&mut self, buf: &[u8]) {
        self.stream.extend(buf.iter().copied());
        self.align += buf.len();
    }
}

macro_rules! from_bits {
    ($v:ident as $t:ident) => {
        $t::from_le_bytes($v.to_le_bytes())
    };
}

impl<'a, T> ser::Serializer for &'a mut Serializer<T>
where
    T: Extend<u8>,
{
    type Ok = &'a T;
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, T>;
    type SerializeTuple = Self;
    type SerializeTupleStruct = SerializeTupleStruct<'a, T>;
    type SerializeTupleVariant = SerializeTupleVariant<'a, T>;
    type SerializeMap = Self;
    type SerializeStruct = SerializeStruct<'a, T>;
    type SerializeStructVariant = SerializeStructVariant<'a, T>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize {}", self.indent, v);
        self.write_u8(u8::from(v));
        Ok(&self.stream)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize i8 {} ({1:#04x})", self.indent, v);
        self.write_u8(from_bits!(v as u8));
        Ok(&self.stream)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize i16 {} ({1:#06x})", self.indent, v);
        self.write_u16(from_bits!(v as u16));
        Ok(&self.stream)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize i32 {} ({1:#010x})", self.indent, v);
        self.write_u32(from_bits!(v as u32));
        Ok(&self.stream)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize i64 {} ({1:#018x})", self.indent, v);
        self.write_u64(from_bits!(v as u64));
        Ok(&self.stream)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize u8 {} ({1:#04x})", self.indent, v);
        self.write_u8(v);
        Ok(&self.stream)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize u16 {} ({1:#06x})", self.indent, v);
        self.write_u16(v);
        Ok(&self.stream)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize u32 {} ({1:#010x})", self.indent, v);
        self.write_u32(v);
        Ok(&self.stream)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize u64 {} ({1:#018x})", self.indent, v);
        self.write_u64(v);
        Ok(&self.stream)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize f32 {} ({:#010x})", self.indent, v, v.to_bits());
        self.write_u32(v.to_bits());
        Ok(&self.stream)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize f64 {} ({:#018x})", self.indent, v, v.to_bits());
        self.write_u64(v.to_bits());
        Ok(&self.stream)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize char {:?}", self.indent, v);
        self.write_u8(v as u8);
        Ok(&self.stream)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize str {:?}", self.indent, v);
        self.write_u32(u32::try_from(v.len() + 1)?);
        self.write_all(v.as_bytes());
        self.write_u8(0);
        Ok(&self.stream)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        trace!(
            "{}serialize bytes ({} byte{})",
            self.indent,
            v.len(),
            if v.len() < 2 { "s" } else { "" }
        );
        self.write_u32(u32::try_from(v.len())?);
        self.write_all(v);
        Ok(&self.stream)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize None", self.indent);
        Ok(&self.stream)
    }

    fn serialize_some<V>(self, value: &V) -> Result<Self::Ok, Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize Some(..)...", self.indent);
        self.indent.push();
        value.serialize(&mut *self)?;
        self.indent.pop();
        trace!("{}serialize Some(..)... done", self.indent);
        Ok(&self.stream)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize ()", self.indent);
        Ok(&self.stream)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize {}", self.indent, name);
        Ok(&self.stream)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        trace!("{}serialize {}::{}", self.indent, name, variant);
        self.write_u32(variant_index);
        Ok(&self.stream)
    }

    fn serialize_newtype_struct<V>(
        self,
        name: &'static str,
        value: &V,
    ) -> Result<Self::Ok, Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize {}...", self.indent, name);
        self.indent.push();
        value.serialize(&mut *self)?;
        self.indent.pop();
        trace!("{}serialize {}... done", self.indent, name);
        Ok(&self.stream)
    }

    fn serialize_newtype_variant<V>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &V,
    ) -> Result<Self::Ok, Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index);
        self.indent.push();
        value.serialize(&mut *self)?;
        self.indent.pop();
        trace!("{}serialize {}::{}... done", self.indent, name, variant);
        Ok(&self.stream)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        trace!("{}serialize seq...", self.indent);
        if let Some(len) = len {
            self.write_u32(u32::try_from(len)?);
        }
        self.indent.push();
        Ok(SerializeSeq { ser: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        trace!("{}serialize tuple...", self.indent);
        self.indent.push();
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        trace!(
            "{}serialize tuple struct {} ({} fields)...",
            self.indent,
            name,
            len
        );
        self.indent.push();
        Ok(Self::SerializeTupleStruct { ser: self, name })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index);
        self.indent.push();
        Ok(Self::SerializeTupleVariant {
            ser: self,
            name,
            variant,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        trace!("{}serialize map...", self.indent);
        if let Some(len) = len {
            self.write_u32(u32::try_from(len)?);
        }
        self.indent.push();
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        trace!("{}serialize struct {}...", self.indent, name);
        self.indent.push();
        Ok(Self::SerializeStruct { ser: self, name })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        trace!("{}serialize {}::{}...", self.indent, name, variant);
        self.write_u32(variant_index);
        self.indent.push();
        Ok(Self::SerializeStructVariant {
            ser: self,
            name,
            variant,
        })
    }
}

pub struct SerializeSeq<'a, T>
where
    T: Extend<u8>,
{
    ser: &'a mut Serializer<T>,
}

impl<'a, T> ser::SerializeSeq for SerializeSeq<'a, T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.ser.indent.pop();
        trace!("{}serialize seq... done", self.ser.indent);
        Ok(&self.ser.stream)
    }
}

impl<'a, T> ser::SerializeTuple for &'a mut Serializer<T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.indent.pop();
        trace!("{}serialize tuple... done", self.indent);
        Ok(&self.stream)
    }
}

pub struct SerializeTupleStruct<'a, T>
where
    T: Extend<u8>,
{
    ser: &'a mut Serializer<T>,
    name: &'static str,
}

impl<'a, T> ser::SerializeTupleStruct for SerializeTupleStruct<'a, T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize field {}...", self.ser.indent, self.name);
        self.ser.indent.push();
        value.serialize(&mut *self.ser)?;
        self.ser.indent.pop();
        trace!("{}serialize field {}... done", self.ser.indent, self.name);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let (ser, name) = (self.ser, self.name);
        ser.indent.pop();
        trace!("{}serialize field {}... done", ser.indent, name);
        Ok(&ser.stream)
    }
}

pub struct SerializeTupleVariant<'a, T>
where
    T: Extend<u8>,
{
    ser: &'a mut Serializer<T>,
    name: &'static str,
    variant: &'static str,
}

impl<'a, T> ser::SerializeTupleVariant for SerializeTupleVariant<'a, T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        value.serialize(&mut *self.ser)?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let (ser, name, variant) = (self.ser, self.name, self.variant);
        ser.indent.pop();
        trace!("{}serialize {}::{}... done", ser.indent, name, variant);
        Ok(&ser.stream)
    }
}

impl<'a, T> ser::SerializeMap for &'a mut Serializer<T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_key<V>(&mut self, key: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize key...", self.indent);
        self.indent.push();
        key.serialize(&mut **self)?;
        self.indent.pop();
        trace!("{}serialize key... done", self.indent);
        Ok(())
    }

    fn serialize_value<V>(&mut self, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!("{}serialize value...", self.indent);
        self.indent.push();
        value.serialize(&mut **self)?;
        self.indent.pop();
        trace!("{}serialize value... done", self.indent);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.indent.pop();
        trace!("{}serialize map... done", self.indent);
        Ok(&self.stream)
    }
}

pub struct SerializeStruct<'a, T>
where
    T: Extend<u8>,
{
    ser: &'a mut Serializer<T>,
    name: &'static str,
}

impl<'a, T> ser::SerializeStruct for SerializeStruct<'a, T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!(
            "{}serialize field {}::{}...",
            self.ser.indent,
            self.name,
            key
        );
        self.ser.indent.push();
        value.serialize(&mut *self.ser)?;
        self.ser.indent.pop();
        trace!(
            "{}serialize field {}::{}... done",
            self.ser.indent,
            self.name,
            key
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let (ser, name) = (self.ser, self.name);
        ser.indent.pop();
        trace!("{}serialize struct {}... done", ser.indent, name);
        Ok(&ser.stream)
    }
}

pub struct SerializeStructVariant<'a, T>
where
    T: Extend<u8>,
{
    ser: &'a mut Serializer<T>,
    name: &'static str,
    variant: &'static str,
}

impl<'a, T> ser::SerializeStructVariant for SerializeStructVariant<'a, T>
where
    T: Extend<u8>,
{
    type Ok = <&'a mut Serializer<T> as ser::Serializer>::Ok;
    type Error = <&'a mut Serializer<T> as ser::Serializer>::Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<(), Self::Error>
    where
        V: ?Sized + Serialize,
    {
        trace!(
            "{}serialize {}::{}::{}...",
            self.ser.indent,
            self.name,
            self.variant,
            key
        );
        value.serialize(&mut *self.ser)?;
        trace!(
            "{}serialize {}::{}::{}... done",
            self.ser.indent,
            self.name,
            self.variant,
            key
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let (ser, name, variant) = (self.ser, self.name, self.variant);
        ser.indent.pop();
        trace!("{}serialize {}::{}... done", ser.indent, name, variant);
        Ok(&ser.stream)
    }
}

/// Serialize a value to CDR bytes.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized.
pub fn to_bytes<T>(le: bool, value: T) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut serializer = Serializer::default().with_le(le);
    value.serialize(&mut serializer).cloned()
}

/// Compute the CDR serialized size of a value.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized or if its size does not
/// fit in 32 bits.
pub fn size_of<T>(value: T) -> Result<u32, Error>
where
    T: Serialize,
{
    #[derive(Default)]
    struct SizeOf(usize);

    impl Extend<u8> for SizeOf {
        fn extend<T>(&mut self, iter: T)
        where
            T: IntoIterator<Item = u8>,
        {
            self.0 += iter.into_iter().count();
        }
    }

    let mut ser = Serializer::<SizeOf>::default();
    let size = value.serialize(&mut ser)?.0;
    Ok(u32::try_from(size)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn length_too_large() {
        struct Seq(usize);

        impl Serialize for Seq {
            fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                ser::SerializeSeq::end(s.serialize_seq(Some(self.0))?)
            }
        }

        assert_eq!(to_bytes(false, Seq(usize::MAX)), Err(Error::TooLarge));
    }

    #[test]
    fn serialize_primitive() {
        assert_eq!(to_bytes(false, false), Ok(vec![0x00]));
        assert_eq!(to_bytes(true, false), Ok(vec![0x00]));
        assert_eq!(to_bytes(false, true), Ok(vec![0x01]));
        assert_eq!(to_bytes(true, true), Ok(vec![0x01]));

        assert_eq!(to_bytes(false, 1_i8), Ok(vec![0x01]));
        assert_eq!(to_bytes(true, 1_i8), Ok(vec![0x01]));
        assert_eq!(to_bytes(false, -2_i8), Ok(vec![0xfe]));
        assert_eq!(to_bytes(true, -2_i8), Ok(vec![0xfe]));

        assert_eq!(to_bytes(false, 1_i16), Ok(vec![0x00, 0x01]));
        assert_eq!(to_bytes(true, 1_i16), Ok(vec![0x01, 0x00]));
        assert_eq!(to_bytes(false, -2_i16), Ok(vec![0xff, 0xfe]));
        assert_eq!(to_bytes(true, -2_i16), Ok(vec![0xfe, 0xff]));

        assert_eq!(to_bytes(false, 1_i32), Ok(vec![0x00, 0x00, 0x00, 0x01]));
        assert_eq!(to_bytes(true, 1_i32), Ok(vec![0x01, 0x00, 0x00, 0x00]));
        assert_eq!(to_bytes(false, -2_i32), Ok(vec![0xff, 0xff, 0xff, 0xfe]));
        assert_eq!(to_bytes(true, -2_i32), Ok(vec![0xfe, 0xff, 0xff, 0xff]));

        assert_eq!(
            to_bytes(false, 1_i64),
            Ok(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01])
        );
        assert_eq!(
            to_bytes(true, 1_i64),
            Ok(vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        );
        assert_eq!(
            to_bytes(false, -2_i64),
            Ok(vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe])
        );
        assert_eq!(
            to_bytes(true, -2_i64),
            Ok(vec![0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
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
            to_bytes(false, &unaligned),
            Ok(vec![
                0x11, 0x00, 0x22, 0x33, 0x44, 0x00, 0x00, 0x00, 0x55, 0x66, 0x77, 0x88
            ])
        );
    }
}
