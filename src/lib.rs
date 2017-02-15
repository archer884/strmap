#[macro_use] extern crate bitflags;
extern crate byteorder;

use std::io;
use std::slice;

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
    offsets: Vec<(u32, u32)>,
}

impl StrMap {
    // I'm sure this can fail, but I am not certain what failure modes can result yet.
    pub fn from_bytes<T: AsRef<[u8]>>(b: T) -> io::Result<StrMap> {
        use byteorder::{NetworkEndian, ReadBytesExt};
        use std::io::Cursor;

        let mut header = Cursor::new(&b);

        let version = header.read_u32::<NetworkEndian>()?;
        let count = header.read_u32::<NetworkEndian>()?;
        let longest = header.read_u32::<NetworkEndian>()?;
        let shortest = header.read_u32::<NetworkEndian>()?;
        let flags = StrFlags::from_bits(header.read_u32::<NetworkEndian>()?).expect("well, dammit");
        let delimiter = header.read_u8()?;

        // If the header lied about the length of our file, this will panic. I'm ok with that.
        // Additionally, we need to read one additional value to get valid offsets, because each 
        // offset consists of a pairing of two offset values--hence our range is `0..(count + 1)`.
        let mut offset_cursor = Cursor::new(&b.as_ref()[(4 * 6)..]);
        let values: Vec<_> = (0..(count + 1)).filter_map(|_| offset_cursor.read_u32::<NetworkEndian>().ok()).collect();
        let offsets: Vec<_> = OffsetsIter::new(values.iter().cloned()).collect();

        if count as usize != offsets.len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Str count in header does not match str count in data: {} vs {}", count, offsets.len())
            ));
        }

        Ok(StrMap {
            version: version,
            count: count,
            longest: longest,
            shortest: shortest,
            flags: flags,
            delimiter: delimiter,
            offsets: offsets,
        })
    }

    /// The number of strings contained in the mapped file.
    pub fn len(&self) -> u32 {
        self.count
    }

    /// The longest string contained in the mapped file.
    pub fn longest(&self) -> u32 {
        self.longest
    }

    /// The shortest string contained in the mapped file.
    pub fn shortest(&self) -> u32 {
        self.shortest
    }

    /// The delimiter used in the mapped file.
    pub fn delimiter(&self) -> u8 {
        self.delimiter
    }

    /// Whether the index for this file is randomized.
    pub fn is_random(&self) -> bool {
        self.flags.contains(STR_RANDOM)
    }

    /// Whether the index for this file is sorted.
    pub fn is_ordered(&self) -> bool {
        self.flags.contains(STR_ORDERED)
    }

    /// Whether the contents of this file have been rotated via rot13.
    pub fn is_rotated(&self) -> bool {
        self.flags.contains(STR_ROTATED)
    }

    /// Returns an iterator over the string offsets contained in this index.
    pub fn iter(&self) -> StrMapIter {
        StrMapIter {
            source: self.offsets.iter()
        }
    }
}

impl<'a> IntoIterator for &'a StrMap {
    type Item = (u32, u32);
    type IntoIter = StrMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StrMapIter {
            source: self.offsets.iter()
        }
    }
}

pub struct StrMapIter<'a> {
    source: slice::Iter<'a, (u32, u32)>
}

impl<'a> Iterator for StrMapIter<'a> {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        self.source.next().map(|&(left, right)| (left, right))
    }
}

struct OffsetsIter<I> {
    source: I,
    last: Option<u32>,
}

impl<I> OffsetsIter<I> {
    fn new(source: I) -> OffsetsIter<I> {
        OffsetsIter {
            source: source,
            last: None,
        }
    }
}

impl<I> Iterator for OffsetsIter<I>
    where I: Iterator<Item = u32>
{
    type Item = (u32, u32);

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.source.next() {
            None => return None,
            Some(item) => item,
        };

        match self.last {
            None => {
                self.last = Some(next);
                return self.next();
            },

            Some(last) => {
                let ret = Some((last, next));
                self.last = Some(next);
                ret
            }
        }
    }
}

#[cfg(test)]
mod tests {
    static SAMPLE: &'static str = include_str!("../sample.txt");
    static SAMPLE_DAT: &'static [u8] = include_bytes!("../sample.txt.dat");

    #[test]
    fn can_parse_sample_file_dat() {
        let x = super::StrMap::from_bytes(SAMPLE_DAT).unwrap();
        let strings: Vec<_> = x.iter().map(|(start, len)| unsafe {
            SAMPLE.slice_unchecked(start as usize, len as usize - 2).trim_right_matches(|c| '%' == c || c.is_whitespace())
        }).collect();

        let expected = &[
            "This file exists",
            "Solely to provide",
            "A known sample file",
            "To use with strfile",
        ];

        assert_eq!(expected, &*strings);
    }
}
