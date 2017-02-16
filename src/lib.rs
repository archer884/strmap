#[macro_use] extern crate bitflags;
extern crate byteorder;

use std::io;
use std::slice;

bitflags! {
    flags StrFlags: u32 {
        const STR_RANDOM = 0b00000001,
        const STR_ORDERED = 0b00000010,
        const STR_ROTATED = 0b00000100,
    }
}

#[derive(Debug)]
pub struct StrMap {
    version: u32,
    count: u32,
    longest: u32,
    shortest: u32,
    flags: StrFlags,
    delimiter: u8,
    offsets: Vec<(u32, u32)>,
}

impl StrMap {
    pub fn read<T: io::Read + io::Seek>(s: &mut T) -> io::Result<StrMap> {
        use byteorder::{NetworkEndian, ReadBytesExt};
        use std::io::SeekFrom;

        let version = s.read_u32::<NetworkEndian>()?;

        if version == 1 {
            return _x64_read(s);
        }

        let count = s.read_u32::<NetworkEndian>()?;
        let longest = s.read_u32::<NetworkEndian>()?;
        let shortest = s.read_u32::<NetworkEndian>()?;
        let flags = StrFlags::from_bits(s.read_u32::<NetworkEndian>()?).expect("well, dammit");
        let delimiter = s.read_u8()?;

        // We need to read one additional value to get valid offsets, because each 
        // offset consists of a pairing of two offset values--hence our range is 
        // `0..(count + 1)`. We begin by skipping the next three bytes, since a 
        // delimiter consists of one byte only.
        s.seek(SeekFrom::Current(3))?;
        let values: Vec<_> = (0..(count + 1)).filter_map(|_| s.read_u32::<NetworkEndian>().ok()).collect();
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

fn _x64_read<T: io::Read + io::Seek>(s: &mut T) -> io::Result<StrMap> {
    use byteorder::{NetworkEndian, ReadBytesExt};
    use std::io::SeekFrom;
    
    s.seek(SeekFrom::Current(4))?;
    let count = s.read_u32::<NetworkEndian>()?;

    s.seek(SeekFrom::Current(4))?;
    let longest = s.read_u32::<NetworkEndian>()?;

    s.seek(SeekFrom::Current(4))?;
    let shortest = s.read_u32::<NetworkEndian>()?;

    s.seek(SeekFrom::Current(4))?;
    let flags = StrFlags::from_bits(s.read_u32::<NetworkEndian>()?).expect("well, dammit");

    s.seek(SeekFrom::Current(4))?;
    let delimiter = s.read_u8()?;

    // For this case, we skip every second record because strfile is worthless
    // on x64 systems. I almost stopped typing at "worthless," but I am nice.
    s.seek(SeekFrom::Current(7))?;
    let values: Vec<_> = (0..(count + 1))
        .filter_map(|_| {
            let ret = s.read_u32::<NetworkEndian>().ok();
            match s.seek(SeekFrom::Current(4)) {
                Err(_) => None,
                _ => ret,
            }
        })
        .collect();

    let offsets: Vec<_> = OffsetsIter::new(values.iter().cloned()).collect();

    if count as usize != offsets.len() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Str count in header does not match str count in data: header({}) vs actual({})", count, offsets.len())
        ));
    }

    Ok(StrMap {
        version: 1, // This is how we got here, remember?
        count: count,
        longest: longest,
        shortest: shortest,
        flags: flags,
        delimiter: delimiter,
        offsets: offsets,
    })
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
    use std::io::Cursor;

    static SAMPLE: &'static str = include_str!("../sample.txt");
    static SAMPLE_DAT: &'static [u8] = include_bytes!("../sample.txt.dat");
    static SAMPLE_DAT_64: &'static [u8] = include_bytes!("../sample.txt-64.dat");

    #[test]
    fn can_parse_x86_dat() {
        parse(SAMPLE_DAT);
    }

    #[test]
    fn can_parse_x64_dat() {
        parse(SAMPLE_DAT_64);
    }

    fn parse(input: &[u8]) {
        let mut x = Cursor::new(input);
        let x = super::StrMap::read(&mut x).unwrap();
        let strings: Vec<_> = x.iter().map(|(start, len)| unsafe {
            SAMPLE
                .slice_unchecked(start as usize, len as usize - 2)
                .trim_right_matches(|c| '%' == c || c.is_whitespace())
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
