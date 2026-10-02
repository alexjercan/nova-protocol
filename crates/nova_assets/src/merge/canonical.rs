//! The canonical byte form the content catalog digest hashes.
//!
//! serde_json cannot stand in for it: it writes a non-finite float as `null`,
//! the same bytes as `None`, and its map keeps insertion order whenever any
//! crate in the build enables `preserve_order` (`nova_bench` does). This form
//! refuses a non-finite float, sorts every map by its encoded key, and tags
//! every value, so two different values never share an encoding.

use std::fmt;

use serde::ser::{self, Serialize};

/// Why a value has no canonical form.
#[derive(Debug)]
pub(super) struct CanonicalFault(String);

impl fmt::Display for CanonicalFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CanonicalFault {}

impl ser::Error for CanonicalFault {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

/// Append the canonical form of `value` to `out`.
///
/// # Errors
///
/// A non-finite float, named by the struct fields that lead to it, or a
/// custom serializer error.
pub(super) fn write_canonical<T: Serialize + ?Sized>(
    value: &T,
    out: &mut Vec<u8>,
) -> Result<(), CanonicalFault> {
    let mut path = Vec::new();
    value.serialize(Canonical {
        out,
        path: &mut path,
    })
}

/// Append `text`, length first, so a text never runs into what follows it.
fn write_text(out: &mut Vec<u8>, text: &str) {
    out.extend((text.len() as u64).to_le_bytes());
    out.extend(text.as_bytes());
}

struct Canonical<'a> {
    out: &'a mut Vec<u8>,
    /// The struct fields from the root to the value being written, for the
    /// fault a non-finite float raises.
    path: &'a mut Vec<&'static str>,
}

impl<'a> Canonical<'a> {
    fn tagged(self, tag: u8, bytes: &[u8]) -> Result<(), CanonicalFault> {
        self.out.push(tag);
        self.out.extend(bytes);
        Ok(())
    }

    fn non_finite(&self, value: impl fmt::Display) -> CanonicalFault {
        let path = if self.path.is_empty() {
            "<root>".to_string()
        } else {
            self.path.join(".")
        };
        CanonicalFault(format!("{path} is {value}, not a finite number"))
    }

    fn open(self, open: u8, close: u8) -> Compound<'a> {
        self.out.push(open);
        Compound {
            out: self.out,
            path: self.path,
            close,
            entries: Vec::new(),
            key: None,
        }
    }

    fn open_variant(self, open: u8, close: u8, variant: &str) -> Compound<'a> {
        self.out.push(open);
        write_text(self.out, variant);
        Compound {
            out: self.out,
            path: self.path,
            close,
            entries: Vec::new(),
            key: None,
        }
    }
}

impl<'a> ser::Serializer for Canonical<'a> {
    type Ok = ();
    type Error = CanonicalFault;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    fn serialize_bool(self, value: bool) -> Result<(), CanonicalFault> {
        self.tagged(b'B', &[u8::from(value)])
    }

    fn serialize_i8(self, value: i8) -> Result<(), CanonicalFault> {
        self.serialize_i128(i128::from(value))
    }

    fn serialize_i16(self, value: i16) -> Result<(), CanonicalFault> {
        self.serialize_i128(i128::from(value))
    }

    fn serialize_i32(self, value: i32) -> Result<(), CanonicalFault> {
        self.serialize_i128(i128::from(value))
    }

    fn serialize_i64(self, value: i64) -> Result<(), CanonicalFault> {
        self.serialize_i128(i128::from(value))
    }

    fn serialize_i128(self, value: i128) -> Result<(), CanonicalFault> {
        self.tagged(b'i', &value.to_le_bytes())
    }

    fn serialize_u8(self, value: u8) -> Result<(), CanonicalFault> {
        self.serialize_u128(u128::from(value))
    }

    fn serialize_u16(self, value: u16) -> Result<(), CanonicalFault> {
        self.serialize_u128(u128::from(value))
    }

    fn serialize_u32(self, value: u32) -> Result<(), CanonicalFault> {
        self.serialize_u128(u128::from(value))
    }

    fn serialize_u64(self, value: u64) -> Result<(), CanonicalFault> {
        self.serialize_u128(u128::from(value))
    }

    fn serialize_u128(self, value: u128) -> Result<(), CanonicalFault> {
        self.tagged(b'u', &value.to_le_bytes())
    }

    fn serialize_f32(self, value: f32) -> Result<(), CanonicalFault> {
        if !value.is_finite() {
            return Err(self.non_finite(value));
        }
        self.tagged(b'f', &value.to_le_bytes())
    }

    fn serialize_f64(self, value: f64) -> Result<(), CanonicalFault> {
        if !value.is_finite() {
            return Err(self.non_finite(value));
        }
        self.tagged(b'd', &value.to_le_bytes())
    }

    fn serialize_char(self, value: char) -> Result<(), CanonicalFault> {
        self.tagged(b'c', &u32::from(value).to_le_bytes())
    }

    fn serialize_str(self, value: &str) -> Result<(), CanonicalFault> {
        self.out.push(b's');
        write_text(self.out, value);
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), CanonicalFault> {
        self.out.push(b'b');
        self.out.extend((value.len() as u64).to_le_bytes());
        self.out.extend(value);
        Ok(())
    }

    fn serialize_none(self) -> Result<(), CanonicalFault> {
        self.tagged(b'N', &[])
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), CanonicalFault> {
        self.out.push(b'S');
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), CanonicalFault> {
        self.tagged(b'U', &[])
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), CanonicalFault> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<(), CanonicalFault> {
        self.out.push(b'V');
        write_text(self.out, variant);
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), CanonicalFault> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), CanonicalFault> {
        self.out.push(b'W');
        write_text(self.out, variant);
        self.path.push(variant);
        value.serialize(Canonical {
            out: &mut *self.out,
            path: &mut *self.path,
        })?;
        self.path.pop();
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open(b'L', b'l'))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open(b'L', b'l'))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open(b'L', b'l'))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open_variant(b'T', b'l', variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open(b'M', b'm'))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open(b'R', b'r'))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Compound<'a>, CanonicalFault> {
        Ok(self.open_variant(b'X', b'r', variant))
    }
}

/// A sequence, tuple, struct or map being written.
struct Compound<'a> {
    out: &'a mut Vec<u8>,
    path: &'a mut Vec<&'static str>,
    /// The closing tag.
    close: u8,
    /// A map's entries, written sorted by encoded key when the map ends.
    entries: Vec<(Vec<u8>, Vec<u8>)>,
    /// A map key waiting for its value.
    key: Option<Vec<u8>>,
}

impl Compound<'_> {
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), CanonicalFault> {
        value.serialize(Canonical {
            out: &mut *self.out,
            path: &mut *self.path,
        })
    }

    fn field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), CanonicalFault> {
        write_text(self.out, name);
        self.path.push(name);
        self.element(value)?;
        self.path.pop();
        Ok(())
    }

    /// Encode `value` on its own, for a map entry sorted at the end.
    fn detached<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<Vec<u8>, CanonicalFault> {
        let mut bytes = Vec::new();
        value.serialize(Canonical {
            out: &mut bytes,
            path: &mut *self.path,
        })?;
        Ok(bytes)
    }

    fn finish(mut self) -> Result<(), CanonicalFault> {
        self.entries.sort();
        for (key, value) in std::mem::take(&mut self.entries) {
            self.out.extend(key);
            self.out.extend(value);
        }
        self.out.push(self.close);
        Ok(())
    }
}

impl ser::SerializeSeq for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeTuple for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeTupleStruct for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeTupleVariant for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.element(value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeMap for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.key = Some(self.detached(key)?);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .key
            .take()
            .ok_or_else(|| CanonicalFault("a map value came before its key".to_string()))?;
        let value = self.detached(value)?;
        self.entries.push((key, value));
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeStruct for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.field(name, value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}

impl ser::SerializeStructVariant for Compound<'_> {
    type Ok = ();
    type Error = CanonicalFault;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.field(name, value)
    }

    fn end(self) -> Result<(), Self::Error> {
        self.finish()
    }
}
