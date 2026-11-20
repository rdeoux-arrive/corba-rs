use super::error::{Error, Result};
use super::indent::Indent;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use log::{trace, warn};
use serde::de::{self, Deserialize, DeserializeSeed, Visitor};

macro_rules! from_bits {
    ($v:expr => $t:ident) => {
        $t::from_le_bytes($v.to_le_bytes())
    };
}

pub struct Deserializer<'a> {
    bytes: &'a [u8],
    align: usize,
    le: bool,
    indent: Indent,
}

impl Deserializer<'_> {
    #[must_use]
    pub const fn with_le(self, le: bool) -> Self {
        Self { le, ..self }
    }
}

impl<'a> From<&'a [u8]> for Deserializer<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            align: 0,
            le: false,
            indent: Indent::default(),
        }
    }
}

/// Deserialize a value from CDR bytes.
///
/// # Errors
///
/// Returns an error if the buffer is truncated or does not hold a valid value.
pub fn from_bytes<'de, T>(buf: &[u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::from(buf);
    T::deserialize(&mut deserializer)
}

impl Deserializer<'_> {
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        if self.align & 1 != 0 {
            let mut buf = [0];
            self.read_exact(&mut buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {buf:?}");
            }
        }
        let mut buf = [0, 0];
        self.read_exact(&mut buf)?;
        Ok(if self.le {
            u16::from_le_bytes(buf)
        } else {
            u16::from_be_bytes(buf)
        })
    }

    fn read_u32(&mut self) -> Result<u32> {
        if self.align & 3 != 0 {
            let buf = &mut [0; 3][..(!self.align + 1) & 3];
            self.read_exact(buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {buf:?}");
            }
        }
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(if self.le {
            u32::from_le_bytes(buf)
        } else {
            u32::from_be_bytes(buf)
        })
    }

    fn read_u64(&mut self) -> Result<u64> {
        if self.align & 7 != 0 {
            let buf = &mut [0; 7][..(!self.align + 1) & 7];
            self.read_exact(buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {buf:?}");
            }
        }
        let mut buf = [0; 8];
        self.read_exact(&mut buf)?;
        Ok(if self.le {
            u64::from_le_bytes(buf)
        } else {
            u64::from_be_bytes(buf)
        })
    }

    /// Read `len` bytes, checking they are available before allocating.
    fn read_vec(&mut self, len: u32) -> Result<Vec<u8>> {
        let len = len as usize;
        if self.bytes.len() < len {
            return Err(Error::Truncated);
        }
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.bytes.len() >= buf.len() {
            buf.copy_from_slice(&self.bytes[..buf.len()]);
            self.bytes = &self.bytes[buf.len()..];
            self.align += buf.len();
            Ok(())
        } else {
            Err(Error::Truncated)
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'_> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize bool...", self.indent);
        let v = match self.read_u8()? {
            0 => false,
            1 => true,
            v => return Err(Error::Bool(v)),
        };
        trace!("{}deserialize bool... {}", self.indent, v);
        visitor.visit_bool(v)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize i8...", self.indent);
        let v = from_bits!(self.read_u8()? => i8);
        trace!("{}deserialize i8... {}", self.indent, v);
        visitor.visit_i8(v)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize i16...", self.indent);
        let v = from_bits!(self.read_u16()? => i16);
        trace!("{}deserialize i16... {}", self.indent, v);
        visitor.visit_i16(v)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize i32...", self.indent);
        let v = from_bits!(self.read_u32()? => i32);
        trace!("{}deserialize i32... {}", self.indent, v);
        visitor.visit_i32(v)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize i64...", self.indent);
        let v = from_bits!(self.read_u64()? => i64);
        trace!("{}deserialize i64... {}", self.indent, v);
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize u8...", self.indent);
        let v = self.read_u8()?;
        trace!("{}deserialize u8... {}", self.indent, v);
        visitor.visit_u8(v)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize u16...", self.indent);
        let v = self.read_u16()?;
        trace!("{}deserialize u16... {}", self.indent, v);
        visitor.visit_u16(v)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize u32...", self.indent);
        let v = self.read_u32()?;
        trace!("{}deserialize u32... {}", self.indent, v);
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize u64...", self.indent);
        let v = self.read_u64()?;
        trace!("{}deserialize u64... {}", self.indent, v);
        visitor.visit_u64(v)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize f32...", self.indent);
        let v = f32::from_bits(self.read_u32()?);
        trace!("{}deserialize f32... {}", self.indent, v);
        visitor.visit_f32(v)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize f64...", self.indent);
        let v = f64::from_bits(self.read_u64()?);
        trace!("{}deserialize f64... {}", self.indent, v);
        visitor.visit_f64(v)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize char...", self.indent);
        let v = self.read_u8()? as char;
        trace!("{}deserialize char... {}", self.indent, v);
        visitor.visit_char(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize str...", self.indent);
        let len = self.read_u32()?;
        let mut buf = self.read_vec(len)?;
        if let Some(terminator) = buf.pop() {
            if terminator == 0 {
                let v = core::str::from_utf8(&buf)?;
                trace!("{}deserialize str... {:?}", self.indent, v);
                visitor.visit_str(v)
            } else {
                Err(Error::StringTerminator(terminator))
            }
        } else {
            Err(Error::StringLength)
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize string...", self.indent);
        let len = self.read_u32()?;
        let mut buf = self.read_vec(len)?;
        if let Some(terminator) = buf.pop() {
            if terminator == 0 {
                let v = core::str::from_utf8(&buf)?;
                trace!("{}deserialize string... {:?}", self.indent, v);
                visitor.visit_string(v.to_string())
            } else {
                Err(Error::StringTerminator(terminator))
            }
        } else {
            Err(Error::StringLength)
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize bytes...", self.indent);
        let len = self.read_u32()?;
        let buf = self.read_vec(len)?;
        trace!(
            "{}deserialize bytes... {} byte{}",
            self.indent,
            len,
            if len > 1 { "s" } else { "" }
        );
        visitor.visit_bytes(&buf)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize byte buf...", self.indent);
        let len = self.read_u32()?;
        let v = self.read_vec(len)?;
        trace!(
            "{}deserialize byte buf... {} byte{}",
            self.indent,
            v.len(),
            if v.len() > 1 { "s" } else { "" }
        );
        visitor.visit_byte_buf(v)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize option (unsupported)", self.indent);
        visitor.visit_none()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize ()", self.indent);
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize unit struct {}", self.indent, name);
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize newtype struct{}...", self.indent, name);
        let v = visitor.visit_newtype_struct(&mut *self);
        trace!("{}deserialize newtype struct {}... done", self.indent, name);
        v
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize seq...", self.indent);
        let len = self.read_u32()?;
        self.indent.push();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            size: Some(len as usize),
        });
        self.indent.pop();
        trace!("{}deserialize seq... done", self.indent);
        v
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize tuple ({} items)...", self.indent, len);
        self.indent.push();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            size: Some(len),
        });
        self.indent.pop();
        trace!("{}deserialize tuple... done", self.indent);
        v
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize tuple struct {}...", self.indent, name);
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            size: Some(len),
        });
        trace!("{}deserialize tuple struct {}... done", self.indent, name);
        v
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct SeqAccess<'a, 'b> {
            de: &'a mut Deserializer<'b>,
            name: &'static str,
            fields: &'a [&'static str],
        }

        impl<'de> de::SeqAccess<'de> for SeqAccess<'_, '_> {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: DeserializeSeed<'de>,
            {
                if let Some((field, fields)) = self.fields.split_first() {
                    self.fields = fields;
                    trace!(
                        "{}deserialize field {}::{}...",
                        self.de.indent, self.name, field
                    );
                    self.de.indent.push();
                    let v = seed.deserialize(&mut *self.de).map(Some);
                    self.de.indent.pop();
                    trace!(
                        "{}deserialize field {}::{}... done",
                        self.de.indent, self.name, field
                    );
                    v
                } else {
                    Ok(None)
                }
            }
        }

        trace!("{}deserialize struct {}...", self.indent, name);
        self.indent.push();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            name,
            fields,
        });
        self.indent.pop();
        trace!("{}deserialize struct {}... done", self.indent, name);
        v
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct VariantAccess;

        impl<'de> de::VariantAccess<'de> for VariantAccess {
            type Error = Error;
            fn unit_variant(self) -> Result<()> {
                Ok(())
            }

            fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
            where
                T: de::DeserializeSeed<'de>,
            {
                unimplemented!()
            }

            fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                unimplemented!()
            }

            fn struct_variant<V>(
                self,
                _fields: &'static [&'static str],
                _visitor: V,
            ) -> Result<V::Value>
            where
                V: de::Visitor<'de>,
            {
                unimplemented!()
            }
        }

        struct EnumAccess<'a, 'b> {
            de: &'a mut Deserializer<'b>,
        }

        impl<'de> de::EnumAccess<'de> for EnumAccess<'_, '_> {
            type Error = Error;
            type Variant = VariantAccess;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
            where
                V: de::DeserializeSeed<'de>,
            {
                seed.deserialize(&mut *self.de).map(|v| (v, VariantAccess))
            }
        }

        trace!("{}deserialize enum {}...", self.indent, name);
        self.indent.push();
        let v = visitor.visit_enum(EnumAccess { de: self });
        self.indent.pop();
        trace!("{}deserialize enum {} done...", self.indent, name);
        v
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize identifier...", self.indent);
        self.indent.push();
        let v = visitor.visit_u32(self.read_u32()?);
        self.indent.pop();
        trace!("{}deserialize identifier done...", self.indent);
        v
    }

    /// # Panics
    ///
    /// This function always panics.
    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

struct SeqAccess<'a, 'b> {
    de: &'a mut Deserializer<'b>,
    size: Option<usize>,
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'_, '_> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.size {
            Some(0) => Ok(None),
            Some(size) => {
                self.size = Some(size - 1);
                seed.deserialize(&mut *self.de).map(Some)
            }
            None => seed.deserialize(&mut *self.de).map(Some),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn huge_length_is_truncated_not_allocated() {
        // claimed length 0xFFFF_FFF0, but no payload
        let buf = [0xff, 0xff, 0xff, 0xf0];
        assert_eq!(from_bytes::<String>(&buf), Err(Error::Truncated));
    }
}
