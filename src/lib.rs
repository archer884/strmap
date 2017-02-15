#[macro_use] extern crate bitflags;
extern crate byteorder;

use std::io;

bitflags! {
    flags StrFlags: u32 {
        const STR_NONE = 0b00000000,
        const STR_RANDOM = 0b00000001,
        const STR_ORDERED = 0b00000010,
        const STR_ROTATED = 0b00000100,
    }
}

#[derive(Debug)]
pub struct StrMap {
    // Because this is stored, for some reason. I have a theory that the disposition of
    // this very first value might tell us something about the format of the reset of the
    // file, but I don't know that.
    version: u32,

    count: u32,

    // Looks like these length values will have to be derived from the difference between
    // consecutive offsets. >.<
    longest: u32,
    shortest: u32,

    // These are stored as a single bitfield, which looks to be a full length unsigned
    // long value that appears just after the rest.
    flags: StrFlags,

    // This looks to be stored as a single byte, albeit at the head of a field of thirty-two
    // bits. I'm not sure what all the padding is for, or if this will always appear on the
    // left of the field or if it can move to the right. Ah, the wonders of not knowing
    // all that much C.
    delimiter: u8,

    // Seriously the only field that matters even slightly. I guess this vector will always
    // begin with a zero? /shrug
    offsets: Vec<usize>,
}

impl StrMap {
    // I'm sure this can fail, but I am not certain what failure modes can result yet.
    fn from_bytes<T: AsRef<[u8]>>(b: T) -> io::Result<StrMap> {
        use byteorder::{NetworkEndian, ReadBytesExt};
        use std::io::Cursor;

        let mut header = Cursor::new(&b);

        Ok(StrMap {
            version: header.read_u32::<NetworkEndian>()?,
            count: header.read_u32::<NetworkEndian>()?,
            longest: header.read_u32::<NetworkEndian>()?,
            shortest: header.read_u32::<NetworkEndian>()?,
            flags: StrFlags::from_bits(header.read_u32::<NetworkEndian>()?).expect("well, dammit"),
            delimiter: header.read_u8()?,
            offsets: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    static SAMPLE: &'static [u8] = include_bytes!("../sample.txt.dat");

    use byteorder::{NetworkEndian, ReadBytesExt};
    use std::io::Cursor;

    #[test]
    fn byte_order_is_big_endian() {
        let mut cursor = Cursor::new(SAMPLE);
        assert_eq!(2, cursor.read_u32::<NetworkEndian>().unwrap());
    }

    #[test]
    fn sample_file_version_is_one_and_numstr_is_two() {
        let mut cursor = Cursor::new(SAMPLE);
        assert_eq!(2, cursor.read_u32::<NetworkEndian>().unwrap());
        assert_eq!(4, cursor.read_u32::<NetworkEndian>().unwrap());
    }

    #[test]
    fn can_parse_sample_header() {
        let x = super::StrMap::from_bytes(SAMPLE).unwrap();
        assert!(false, "{:?}", x);
    }
}
