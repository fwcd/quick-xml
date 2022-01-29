use crate::{
    de::{escape::EscapedDeserializer, BorrowingReader, DeEvent, Deserializer, PRIMITIVE_PREFIX},
    errors::serialize::DeError,
};
use serde::de::{self, DeserializeSeed, Deserializer as SerdeDeserializer, Visitor};
use std::borrow::Cow;

/// An enum access
pub struct EnumAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    de: &'a mut Deserializer<'de, R>,
    /// list of variants that are serialized in a primitive fashion (defined as starting with $primitive=)
    primitive_variants: Vec<&'static [u8]>,
}

impl<'de, 'a, R> EnumAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    pub fn new(
        de: &'a mut Deserializer<'de, R>,
        variants: &'static [&'static str],
    ) -> Self {
        EnumAccess {
            de,
            primitive_variants: variants
                .iter()
                .filter(|v| v.starts_with(PRIMITIVE_PREFIX))
                .map(|v| v.as_bytes())
                .collect()
        }
    }
}

impl<'de, 'a, R> de::EnumAccess<'de> for EnumAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;
    type Variant = VariantAccess<'de, 'a, R>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantAccess<'de, 'a, R>), DeError>
    where
        V: DeserializeSeed<'de>,
    {
        let decoder = self.de.reader.decoder();
        let de = match self.de.peek()? {
            DeEvent::Text(t) => {
                let text: &[u8] = t;
                let variant = self.primitive_variants
                    .iter()
                    .map(|&v| v)
                    .find(|v| text == &v[PRIMITIVE_PREFIX.len()..])
                    .unwrap_or(text);
                EscapedDeserializer::new(Cow::Borrowed(variant), decoder, true, self.primitive_variants)},
            DeEvent::Start(e) => EscapedDeserializer::new(Cow::Borrowed(e.name()), decoder, false, self.primitive_variants),
            _ => {
                return Err(DeError::Unsupported(
                    "Invalid event for Enum, expecting `Text` or `Start`",
                ))
            }
        };
        let name = seed.deserialize(de)?;
        Ok((name, VariantAccess { de: self.de }))
    }
}

pub struct VariantAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    de: &'a mut Deserializer<'de, R>,
}

impl<'de, 'a, R> de::VariantAccess<'de> for VariantAccess<'de, 'a, R>
where
    R: BorrowingReader<'de>,
{
    type Error = DeError;

    fn unit_variant(self) -> Result<(), DeError> {
        match self.de.next()? {
            DeEvent::Start(e) => self.de.read_to_end(e.name()),
            DeEvent::Text(_) => Ok(()),
            _ => unreachable!(),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_struct("", fields, visitor)
    }
}
