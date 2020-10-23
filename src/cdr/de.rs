use super::error::{Error, Result};
use log::{trace, warn};
use serde::de::{self, Deserialize, DeserializeSeed, Visitor};
use std::io::{self, Read};

macro_rules! from_bits {
    ($v:expr => $t:ident) => { $t::from_le_bytes($v.to_le_bytes())};
}

pub struct Deserializer<R>
where
    R: Read,
{
    reader: R,
    align: usize,
    le: bool,
    indent: String,
}

impl<R> Deserializer<R>
where
    R: Read,
{
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader,
            align: 0,
            le: false,
            indent: String::new(),
        }
    }

    pub fn with_le(self, le: bool) -> Self {
        Self {
            reader: self.reader,
            align: self.align,
            le,
            indent: self.indent,
        }
    }

    pub fn reader(&self) -> &R {
        &self.reader
    }

    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    fn push_indent(&mut self) {
        self.indent.push_str("  ")
    }

    fn pop_indent(&mut self) {
        self.indent.truncate(self.indent.len() - 2)
    }
}

pub fn from_bytes<'de, T>(buf: &[u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    from_reader(buf)
}

pub fn from_reader<'de, R, T>(reader: R) -> Result<T>
where
    R: Read,
    T: Deserialize<'de>,
{
    let mut deserializer = Deserializer::from_reader(reader);
    T::deserialize(&mut deserializer)
}

impl<R> Deserializer<R>
where
    R: Read,
{
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> io::Result<u16> {
        if self.align & 1 != 0 {
            let mut buf = [0];
            self.read_exact(&mut buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {:?}", buf)
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

    fn read_u32(&mut self) -> io::Result<u32> {
        if self.align & 3 != 0 {
            let mut buf = &mut [0; 3][..(!self.align + 1) & 3];
            self.read_exact(&mut buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {:?}", buf)
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

    fn read_u64(&mut self) -> io::Result<u64> {
        if self.align & 7 != 0 {
            let mut buf = &mut [0; 7][..(!self.align + 1) & 7];
            self.read_exact(&mut buf)?;
            if buf[0] != 0 {
                warn!("found non-zero in padding: {:?}", buf)
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

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_exact(buf)?;
        self.align += buf.len();
        Ok(())
    }
}

impl<'a, 'de, R> de::Deserializer<'de> for &'a mut Deserializer<R>
where
    R: Read,
{
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
        let mut buf = vec![0; len as usize];
        if len > 0 {
            self.read_exact(buf.as_mut())?;
        }
        if let Some(terminator) = buf.pop() {
            if terminator == 0 {
                let v = std::str::from_utf8(&buf)?;
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
        let mut buf = vec![0; len as usize];
        if len > 0 {
            self.read_exact(buf.as_mut())?;
        }
        if let Some(terminator) = buf.pop() {
            if terminator == 0 {
                let v = std::str::from_utf8(&buf)?;
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
        let mut buf = vec![0; len as usize];
        if len > 0 {
            self.read_exact(buf.as_mut())?;
        };
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
        let v = if len == 0 {
            Vec::with_capacity(0)
        } else {
            let mut buf = vec![0; len as usize];
            self.read_exact(buf.as_mut())?;
            buf
        };
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
        self.push_indent();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            size: Some(len as usize),
        });
        self.pop_indent();
        trace!("{}deserialize seq... done", self.indent);
        v
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize tuple ({} items)...", self.indent, len);
        self.push_indent();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            size: Some(len),
        });
        self.pop_indent();
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
        struct SeqAccess<'a, R>
        where
            R: Read,
        {
            de: &'a mut Deserializer<R>,
            name: &'static str,
            fields: &'a [&'static str],
        }

        impl<'a, 'de, R> de::SeqAccess<'de> for SeqAccess<'a, R>
        where
            R: Read,
        {
            type Error = Error;
            fn next_element_seed<T>(
                &mut self,
                seed: T,
            ) -> std::result::Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if let Some((field, fields)) = self.fields.split_first() {
                    self.fields = fields;
                    trace!(
                        "{}deserialize field {}::{}...",
                        self.de.indent,
                        self.name,
                        field
                    );
                    self.de.push_indent();
                    let v = seed.deserialize(&mut *self.de).map(Some);
                    self.de.pop_indent();
                    trace!(
                        "{}deserialize field {}::{}... done",
                        self.de.indent,
                        self.name,
                        field
                    );
                    v
                } else {
                    Ok(None)
                }
            }
        }

        trace!("{}deserialize struct {}...", self.indent, name);
        self.push_indent();
        let v = visitor.visit_seq(SeqAccess {
            de: self,
            name,
            fields,
        });
        self.pop_indent();
        trace!("{}deserialize struct {}... done", self.indent, name);
        v
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        trace!("{}deserialize enum {}...", self.indent, name);
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

struct SeqAccess<'a, R>
where
    R: Read,
{
    de: &'a mut Deserializer<R>,
    size: Option<usize>,
}

impl<'a, 'de, R> de::SeqAccess<'de> for SeqAccess<'a, R>
where
    R: Read,
{
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
