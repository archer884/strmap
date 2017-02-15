#[macro_use] extern crate bitflags;
extern crate byteorder;

bitflags! {
    flags StrFlags: usize {
        const STR_RANDOM = 0b00000001,
        const STR_ORDERED = 0b00000010,
        const STR_ROTATED = 0b00000100,
    }
}

pub struct StrMap {
    // Because this is stored, for some reason. I have a theory that the disposition of 
    // this very first value might tell us something about the format of the reset of the 
    // file, but I don't know that.
    version: u32,

    count: usize,

    // Looks like these length values will have to be derived from the difference between 
    // consecutive offsets. >.<
    longest: usize,
    shortest: usize,

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

#[cfg(test)]
mod tests {
    static SAMPLE: &'static [u8] = include_bytes!("../sample.dat");

    use byteorder::{BigEndian, ReadBytesExt};
    use std::io::Cursor;

    #[test]
    fn byte_order_is_big_endian() {
        let mut cursor = Cursor::new(SAMPLE);
        assert_eq!(1, cursor.read_u32::<BigEndian>().unwrap());
    }

    #[test]
    fn sample_file_version_is_one_and_numstr_is_two() {
        let mut cursor = Cursor::new(SAMPLE);
        assert_eq!(1, cursor.read_u32::<BigEndian>().unwrap());
        assert_eq!(2, cursor.read_u32::<BigEndian>().unwrap());
    }
}
