use std::io;
use byteorder::{LittleEndian, ByteOrder};
use super::{Error, Deserialize, Serialize};

/// Unsigned variable-length integer, limited to 32 bits,
/// represented by at most 5 bytes that may contain padding 0x80 bytes.
#[derive(Copy, Clone)]
pub struct VarUint32(u32);

impl From<VarUint32> for usize {
    fn from(var: VarUint32) -> usize {
        var.0 as usize
    }
}

impl From<VarUint32> for u32 {
    fn from(var: VarUint32) -> u32 {
        var.0
    }
}

impl From<u32> for VarUint32 {
    fn from(i: u32) -> VarUint32 {
        VarUint32(i)
    }
}

impl From<usize> for VarUint32 {
    fn from(i: usize) -> VarUint32 {
        assert!(i <= ::std::u32::MAX as usize);
        VarUint32(i as u32)
    }
}

impl Deserialize for VarUint32 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut res = 0;
        let mut shift = 0;
        let mut u8buf = [0u8; 1];
        loop {
            reader.read_exact(&mut u8buf)?;
            let b = u8buf[0] as u32;
            res |= (b & 0x7f) << shift;
            shift += 7;
            if (b >> 7) == 0 {
                break;
            }
        }
        Ok(VarUint32(res))
    }
}

impl Serialize for VarUint32 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 1];
        let mut v = self.0;
        loop {
            buf[0] = (v & 0b0111_1111) as u8;
            v >>= 7;
            if v > 0 {
                buf[0] |= 0b1000_0000;
            }
            writer.write_all(&buf[..])?;
            if v == 0 { break; }
        }

        Ok(())
    }
}

/// Unsigned variable-length integer, limited to 64 bits,
/// represented by at most 9 bytes that may contain padding 0x80 bytes.
#[derive(Copy, Clone)]
pub struct VarUint64(u64);

impl From<VarUint64> for u64 {
    fn from(var: VarUint64) -> u64 {
        var.0
    }
}

impl Deserialize for VarUint64 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut res = 0;
        let mut shift = 0;
        let mut u8buf = [0u8; 1];
        loop {
            reader.read_exact(&mut u8buf)?;
            let b = u8buf[0] as u64;
            res |= (b & 0x7f) << shift;
            shift += 7;
            if (b >> 7) == 0 {
                break;
            }
        }
        Ok(VarUint64(res))
    }
}

impl Serialize for VarUint64 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 1];
        let mut v = self.0;
        loop {
            buf[0] = (v & 0b0111_1111) as u8;
            v >>= 7;
            if v > 0 {
                buf[0] |= 0b1000_0000;
            }
            writer.write_all(&buf[..])?;
            if v == 0 { break; }
        }

        Ok(())
    }
}

impl From<u64> for VarUint64 {
    fn from(u: u64) -> VarUint64 {
        VarUint64(u)
    }
}

/// 7-bit unsigned integer, encoded in LEB128 (always 1 byte length)
#[derive(Copy, Clone)]
pub struct VarUint7(u8);

impl From<VarUint7> for u8 {
    fn from(v: VarUint7) -> u8 {
        v.0
    }
}

impl From<u8> for VarUint7 {
    fn from(v: u8) -> Self {
        VarUint7(v)
    }
}

impl Deserialize for VarUint7 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut u8buf = [0u8; 1];
        reader.read_exact(&mut u8buf)?;
        Ok(VarUint7(u8buf[0]))
    }
}

impl Serialize for VarUint7 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        // todo check range?
        writer.write_all(&[self.0])?;
        Ok(())
    }
}

/// 7-bit signed integer, encoded in LEB128 (always 1 byte length)
#[derive(Copy, Clone)]
pub struct VarInt7(i8);

impl From<VarInt7> for i8 {
    fn from(v: VarInt7) -> i8 {
        v.0
    }
}

impl From<i8> for VarInt7 {
    fn from(v: i8) -> VarInt7 {
        VarInt7(v)
    }
}

impl Deserialize for VarInt7 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut u8buf = [0u8; 1];
        reader.read_exact(&mut u8buf)?;
        // expand sign
        if u8buf[0] & 0b0100_0000 == 0b0100_0000 { u8buf[0] |= 0b1000_0000 }
        // todo check range
        Ok(VarInt7(u8buf[0] as i8))
    }
}

impl Serialize for VarInt7 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        // todo check range?
        let mut b: u8 = self.0 as u8;
        if self.0 < 0 { b |= 0b0100_0000; b &= 0b0111_1111; }
        writer.write_all(&[b])?;
        Ok(())
    }
}

/// 32-bit signed integer, encoded in LEB128 (can be 1-5 bytes length)
#[derive(Copy, Clone)]
pub struct VarInt32(i32);

impl From<VarInt32> for i32 {
    fn from(v: VarInt32) -> i32 {
        v.0
    }
}

impl From<i32> for VarInt32 {
    fn from(v: i32) -> VarInt32 {
        VarInt32(v)
    }
}

impl Deserialize for VarInt32 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut res = 0;
        let mut shift = 0;
        let mut u8buf = [0u8; 1];
        loop {
            if shift > 31 { return Err(Error::InvalidVarInt32); }
            reader.read_exact(&mut u8buf)?;
            let b = u8buf[0];

            res |= ((b & 0x7f) as i32) << shift;
            shift += 7;
            if (b >> 7) == 0 {
                if shift < 32 && b & 0b0100_0000 == 0b0100_0000 {
                    res |= (1i32 << shift).wrapping_neg();
                }
                break;
            }
        }
        Ok(VarInt32(res))
    }
}

impl Serialize for VarInt32 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 1];
        let mut v = self.0;
        let mut more = true;
        while more {
            buf[0] = (v & 0b0111_1111) as u8;
            v >>= 7;
            if (v == 0 && buf[0] & 0b0100_0000 == 0) || (v == -1 && buf[0] & 0b0100_0000 == 0b0100_0000)  {
                more = false
            } else {
                buf[0] |= 0b1000_0000
            }

            writer.write_all(&buf[..])?;
        }

        Ok(())
    }
}

/// 64-bit signed integer, encoded in LEB128 (can be 1-9 bytes length)
#[derive(Copy, Clone)]
pub struct VarInt64(i64);

impl From<VarInt64> for i64 {
    fn from(v: VarInt64) -> i64 {
        v.0
    }
}

impl From<i64> for VarInt64 {
    fn from(v: i64) -> VarInt64 {
        VarInt64(v)
    }
}

impl Deserialize for VarInt64 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut res = 0i64;
        let mut shift = 0;
        let mut u8buf = [0u8; 1];
        loop {
            if shift > 63 { return Err(Error::InvalidVarInt64); }
            reader.read_exact(&mut u8buf)?;
            let b = u8buf[0];

            res |= ((b & 0x7f) as i64) << shift;
            shift += 7;
            if (b >> 7) == 0 {
                if shift < 64 && b & 0b0100_0000 == 0b0100_0000 {
                    res |= (1i64 << shift).wrapping_neg();
                }
                break;
            }
        }
        Ok(VarInt64(res))
    }
}

impl Serialize for VarInt64 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 1];
        let mut v = self.0;
        let mut more = true;
        while more {
            buf[0] = (v & 0b0111_1111) as u8;
            v >>= 7;
            if (v == 0 && buf[0] & 0b0100_0000 == 0) || (v == -1 && buf[0] & 0b0100_0000 == 0b0100_0000)  {
                more = false
            } else {
                buf[0] |= 0b1000_0000
            }

            writer.write_all(&buf[..])?;
        }

        Ok(())
    }
}

/// 32-bit unsigned integer, encoded in little endian
#[derive(Copy, Clone)]
pub struct Uint32(u32);

impl Deserialize for Uint32 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        // todo check range
        Ok(Uint32(LittleEndian::read_u32(&buf)))
    }
}

impl From<Uint32> for u32 {
    fn from(var: Uint32) -> u32 {
        var.0
    }
}

impl Serialize for Uint32 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 4];
        LittleEndian::write_u32(&mut buf, self.0);
        writer.write_all(&buf)?;
        Ok(())
    }
}

impl From<u32> for Uint32 {
    fn from(u: u32) -> Self { Uint32(u) }
}

/// 64-bit unsigned integer, encoded in little endian
#[derive(Copy, Clone)]
pub struct Uint64(u64);

impl Deserialize for Uint64 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        // todo check range
        Ok(Uint64(LittleEndian::read_u64(&buf)))
    }
}

impl Serialize for Uint64 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buf = [0u8; 8];
        LittleEndian::write_u64(&mut buf, self.0);
        writer.write_all(&buf)?;
        Ok(())
    }
}

impl From<u64> for Uint64 {
    fn from(u: u64) -> Self { Uint64(u) }
}

impl From<Uint64> for u64 {
    fn from(var: Uint64) -> u64 {
        var.0
    }
}


/// VarUint1, 1-bit value (0/1)
#[derive(Copy, Clone)]
pub struct VarUint1(bool);

impl From<VarUint1> for bool {
    fn from(v: VarUint1) -> bool {
        v.0
    }
}

impl From<bool> for VarUint1 {
    fn from(b: bool) -> Self {
        VarUint1(b)
    }
}

impl Deserialize for VarUint1 {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let mut u8buf = [0u8; 1];
        reader.read_exact(&mut u8buf)?;
        match u8buf[0] {
            0 => Ok(VarUint1(false)),
            1 => Ok(VarUint1(true)),
            v @ _ => Err(Error::InvalidVarUint1(v)),
        }
    }
}

impl Serialize for VarUint1 {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&[
            if self.0 { 1u8 } else { 0u8 }
        ])?;
        Ok(())
    }
}

impl Deserialize for String {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let length = VarUint32::deserialize(reader)?.into();
        if length > 0 {
            let mut buf = vec![0u8; length];
            reader.read_exact(&mut buf)?;
            String::from_utf8(buf).map_err(|_| Error::NonUtf8String)
        }
        else {
            Ok(String::new())
        }
    }
}

impl Serialize for String {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Error> {
        VarUint32::from(self.len()).serialize(writer)?;
        writer.write_all(&self.into_bytes()[..])?;
        Ok(())
    }
}

/// List for reading sequence of elements typed `T`, given
/// they are preceded by length (serialized as VarUint32)
pub struct CountedList<T: Deserialize>(Vec<T>);

impl<T: Deserialize> CountedList<T> {
    /// Destroy counted list returing inner vector.
    pub fn into_inner(self) -> Vec<T> { self.0 }
}

impl<T: Deserialize> Deserialize for CountedList<T> where T::Error: From<Error> {
    type Error = T::Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let count: usize = VarUint32::deserialize(reader)?.into();
        let mut result = Vec::new();
        for _ in 0..count { result.push(T::deserialize(reader)?); }
        Ok(CountedList(result))
    }
}

/// Helper struct to write payload which is preceded by
/// it's own length in bytes.
pub struct CountedWriter<'a, W: 'a + io::Write> {
    writer: &'a mut W,
    data: Vec<u8>,
}

impl<'a, W: 'a + io::Write> CountedWriter<'a, W> {
    /// New counted writer on top of the given serial writer
    pub fn new(writer: &'a mut W) -> Self {
        CountedWriter {
            writer: writer,
            data: Vec::new(),
        }
    }

    /// Finish counted writer routing, which writes accumulated length
    /// and actual payload.
    pub fn done(self) -> io::Result<()> {
        let writer = self.writer;
        let data = self.data;
        VarUint32::from(data.len())
            .serialize(writer)
            .map_err(
                |_| io::Error::new(
                    io::ErrorKind::Other,
                    "Length serialization error",
                )
            )?;
        writer.write_all(&data[..])?;
        Ok(())
    }
}

impl<'a, W: 'a + io::Write> io::Write for CountedWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.extend(buf.to_vec());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Helper struct to write series of `T` preceded by the length of the sequence
/// serialized as VarUint32
pub struct CountedListWriter<I: Serialize<Error=::elements::Error>, T: IntoIterator<Item=I>>(pub usize, pub T);

impl<I: Serialize<Error=::elements::Error>, T: IntoIterator<Item=I>> Serialize for CountedListWriter<I, T> {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let len_us = self.0;
        let data = self.1;
        let len: VarUint32 = len_us.into();
        len.serialize(writer)?;
        for data_element in data { data_element.serialize(writer)? }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::super::{deserialize_buffer, Serialize};
    use super::{CountedList, VarInt7, VarUint32, VarInt32, VarInt64, VarUint64};

    fn varuint32_ser_test(val: u32, expected: Vec<u8>) {
        let mut buf = Vec::new();
        let v1: VarUint32 = val.into();
        v1.serialize(&mut buf).expect("to be serialized ok");
        assert_eq!(expected, buf);
    }

    fn varuint32_de_test(dt: Vec<u8>, expected: u32) {
        let val: VarUint32 = super::super::deserialize_buffer(dt).expect("buf to be serialized");
        assert_eq!(expected, val.into());
    }

    fn varuint32_serde_test(dt: Vec<u8>, val: u32) {
        varuint32_de_test(dt.clone(), val);
        varuint32_ser_test(val, dt);
    }

    fn varint32_ser_test(val: i32, expected: Vec<u8>) {
        let mut buf = Vec::new();
        let v1: VarInt32 = val.into();
        v1.serialize(&mut buf).expect("to be serialized ok");
        assert_eq!(expected, buf);
    }

    fn varint32_de_test(dt: Vec<u8>, expected: i32) {
        let val: VarInt32 = super::super::deserialize_buffer(dt).expect("buf to be serialized");
        assert_eq!(expected, val.into());
    }

    fn varint32_serde_test(dt: Vec<u8>, val: i32) {
        varint32_de_test(dt.clone(), val);
        varint32_ser_test(val, dt);
    }

    fn varuint64_ser_test(val: u64, expected: Vec<u8>) {
        let mut buf = Vec::new();
        let v1: VarUint64 = val.into();
        v1.serialize(&mut buf).expect("to be serialized ok");
        assert_eq!(expected, buf);
    }

    fn varuint64_de_test(dt: Vec<u8>, expected: u64) {
        let val: VarUint64 = super::super::deserialize_buffer(dt).expect("buf to be serialized");
        assert_eq!(expected, val.into());
    }

    fn varuint64_serde_test(dt: Vec<u8>, val: u64) {
        varuint64_de_test(dt.clone(), val);
        varuint64_ser_test(val, dt);
    }

    fn varint64_ser_test(val: i64, expected: Vec<u8>) {
        let mut buf = Vec::new();
        let v1: VarInt64 = val.into();
        v1.serialize(&mut buf).expect("to be serialized ok");
        assert_eq!(expected, buf);
    }

    fn varint64_de_test(dt: Vec<u8>, expected: i64) {
        let val: VarInt64 = super::super::deserialize_buffer(dt).expect("buf to be serialized");
        assert_eq!(expected, val.into());
    }

    fn varint64_serde_test(dt: Vec<u8>, val: i64) {
        varint64_de_test(dt.clone(), val);
        varint64_ser_test(val, dt);
    }

    #[test]
    fn varuint32_0() {
        varuint32_serde_test(vec![0u8; 1], 0);
    }

    #[test]
    fn varuint32_1() {
        varuint32_serde_test(vec![1u8; 1], 1);
    }

    #[test]
    fn varuint32_135() {
        varuint32_serde_test(vec![135u8, 0x01], 135);
    }

    #[test]
    fn varuint32_8192() {
        varuint32_serde_test(vec![0x80, 0x40], 8192);
    }

    #[test]
    fn varint32_8192() {
        varint32_serde_test(vec![0x80, 0xc0, 0x00], 8192);
    }

    #[test]
    fn varint32_neg_8192() {
        varint32_serde_test(vec![0x80, 0x40], -8192);
    }

    #[test]
    fn varuint64_0() {
        varuint64_serde_test(vec![0u8; 1], 0);
    }

    #[test]
    fn varuint64_1() {
        varuint64_serde_test(vec![1u8; 1], 1);
    }

    #[test]
    fn varuint64_135() {
        varuint64_serde_test(vec![135u8, 0x01], 135);
    }

    #[test]
    fn varuint64_8192() {
        varuint64_serde_test(vec![0x80, 0x40], 8192);
    }

    #[test]
    fn varint64_8192() {
        varint64_serde_test(vec![0x80, 0xc0, 0x00], 8192);
    }

    #[test]
    fn varint64_neg_8192() {
        varint64_serde_test(vec![0x80, 0x40], -8192);
    }

    #[test]
    fn varint64_min() {
        varint64_serde_test(
            vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7f],
            -9223372036854775808,
        );
    }

    #[test]
    fn varint64_max() {
        varint64_serde_test(
            vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00],
            9223372036854775807,
        );
    }

    #[test]
    fn varint32_min() {
        varint32_serde_test(
            vec![0x80, 0x80, 0x80, 0x80, 0x78],
            -2147483648,
        );
    }

    #[test]
    fn varint32_max() {
        varint32_serde_test(
            vec![0xff, 0xff, 0xff, 0xff, 0x07],
            2147483647,
        );
    }


    #[test]
    fn counted_list() {
        let payload = vec![
            133u8, //(128+5), length is 5
                0x80, 0x80, 0x80, 0x0, // padding
            0x01,
            0x7d,
            0x05,
            0x07,
            0x09,
        ];

        let list: CountedList<VarInt7> =
            deserialize_buffer(payload).expect("type_section be deserialized");

        let vars = list.into_inner();
        assert_eq!(5, vars.len());
        let v3: i8 = (*vars.get(1).unwrap()).into();
        assert_eq!(-0x03i8, v3);
    }
}
