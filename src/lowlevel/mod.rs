use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use reader::{ReadError, SampleIteratorFormat};
use std::any::TypeId;
use crate::WavFileDesc;

pub mod reader;
pub mod writer;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum AudioFormat {
    PCMLinear,
    PCMFloat,
    Unknown(u16),
}

impl AudioFormat {
    pub fn from_u16(d: u16) -> Self {
        match d {
            1 => Self::PCMLinear,
            3 => Self::PCMFloat,
            x => Self::Unknown(x),
        }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), writer::WriteError> {
        writer.write_u16::<LittleEndian>(match self {
            Self::PCMLinear => 1,
            Self::PCMFloat => 3,
            &Self::Unknown(x) => x,
        })?;
        Ok(())
    }
}

macro_rules! type_eq {
    ($t1:ty, $t2:ty) => {TypeId::of::<$t1>() == TypeId::of::<$t2>()};
}

impl AudioFormat {
    pub(crate) fn from_type<T: 'static>() -> Self {
        if type_eq!(T, f32) || type_eq!(T, f64) {
            Self::PCMFloat
        } else if type_eq!(T, u8) || type_eq!(T, i16) || type_eq!(T, i32) || type_eq!(T, i64) {
            Self::PCMLinear
        } else {
            Self::Unknown(0)
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct WavHeader {
    /// File size in bytes, minus 8 byutes
    pub file_size: u32,
    /// Audio format.
    pub audio_format: AudioFormat,
    /// Number of channels
    pub channels: u16,
    /// Sample rate
    pub sample_rate: u32,
    /// Data throughput for real-time playback (byte/s)
    pub bytes_per_sec: u32,
    /// Block size in bytes
    pub bytes_per_block: u16,
    /// Bitsize of the sample type
    pub bits_per_sample: u16,
    /// Data block size (bytes)
    pub data_size: u32,
}

macro_rules! expect_magic {
    (read $reader:expr, $value:expr, $tr:expr) => {
        let val = $reader.read_u32::<LittleEndian>()?.to_le_bytes();
        if &val != $value {
            let found = String::from_utf8_lossy(&val).to_string();
            return Err($tr(found))
        }
    };
}

impl WavHeader {
    pub fn from_reader<R: Read>(mut reader: R) -> Result<(Self, R), ReadError> {
        expect_magic!(read reader, b"RIFF", ReadError::ExpectedRIFF);

        let file_size = reader.read_u32::<LittleEndian>()?;

        expect_magic!(read reader, b"WAVE", ReadError::ExpectedWAVE);
        expect_magic!(read reader, b"fmt ", ReadError::ExpectedFmt);

        let _ = reader.read_u32::<LittleEndian>()?; // Discard the subckunk size
        let audio_format = AudioFormat::from_u16(reader.read_u16::<LittleEndian>()?);
        let channels = reader.read_u16::<LittleEndian>()?;
        let sample_rate = reader.read_u32::<LittleEndian>()?;
        let bytes_per_sec = reader.read_u32::<LittleEndian>()?;
        let bytes_per_block = reader.read_u16::<LittleEndian>()?;
        let bits_per_sample = reader.read_u16::<LittleEndian>()?;

        expect_magic!(read reader, b"data", ReadError::ExpectedData);
        let data_size = reader.read_u32::<LittleEndian>()?;

        Ok((Self {
            file_size,
            audio_format,
            channels,
            sample_rate,
            bytes_per_sec,
            bytes_per_block,
            bits_per_sample,
            data_size,
        }, reader))
    }

    /// /!\ Does not write the data size (unknown for the header)
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), writer::WriteError> {
        writer.write(b"RIFF")?;
        writer.write_u32::<LittleEndian>(self.file_size)?;
        writer.write(b"WAVEfmt ")?;
        writer.write_u32::<LittleEndian>(16)?;
        self.audio_format.write(writer)?;
        writer.write_u16::<LittleEndian>(self.channels)?;
        writer.write_u32::<LittleEndian>(self.sample_rate)?;
        writer.write_u32::<LittleEndian>(self.bytes_per_sec)?;
        writer.write_u16::<LittleEndian>(self.bytes_per_block)?;
        writer.write_u16::<LittleEndian>(self.bits_per_sample)?;

        writer.write(b"data")?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WavFile<R> {
    header: WavHeader,
    pub(crate) data: R,
}

impl<R: Read> WavFile<R> {
    pub fn from_reader(mut reader: R) -> Result<Self, ReadError> {
        let (header, reader) = WavHeader::from_reader(reader)?;
        Ok(Self {
            header,
            data: reader,
        })
    }

    pub fn len(&self) -> usize {
        (self.header.data_size * 8 / self.header.data_size) as usize
    }

    pub fn samples(self) -> Result<SampleIteratorFormat<R>, ()> {
        use reader::SampleIteratorFormat::*;
        use AudioFormat::*;
        use AudioFormat::{PCMLinear, PCMFloat};
        use reader::WavSampleIterator;
        use std::marker::PhantomData;
        let val = match (self.header.bytes_per_block / self.header.channels, self.header.audio_format) {
            (1, PCMLinear) => U8(WavSampleIterator { file: self, __type: PhantomData }),
            (2, PCMLinear) => I16(WavSampleIterator { file: self, __type: PhantomData }),
            #[cfg(feature = "dasp")]
            (3, PCMLinear) => I24(WavSampleIterator { file: self, __type: PhantomData }),
            (4, PCMLinear) => I32(WavSampleIterator { file: self, __type: PhantomData }),
            (4, PCMFloat) => F32(WavSampleIterator { file: self, __type: PhantomData }),
            #[cfg(feature = "dasp")]
            (6, PCMLinear) => I48(WavSampleIterator { file: self, __type: PhantomData }),
            (8, PCMLinear) => I64(WavSampleIterator { file: self, __type: PhantomData }),
            (8, PCMFloat) => F64(WavSampleIterator { file: self, __type: PhantomData }),
            _ => return Err(())
        };
        Ok(val)
    }
}

impl<W: Write> WavFile<W> {
    pub fn write<T: 'static>(desc: WavFileDesc<T>, writer: W) -> Self {
        Self {
            header: desc.into(),
            data: writer,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::Path;

    use crate::lowlevel::{AudioFormat, WavHeader};
    use crate::sample::NumIO;
    use crate::DATA_DIR;

    #[test]
    fn test_header_well_formed() {
        let filename = Path::join(DATA_DIR.as_ref(), "single_sample_u16.wav");
        println!("Filename: {}", filename.display());
        let reader = File::open(filename).unwrap();
        let res = WavHeader::from_reader(reader);
        println!("{:#?}", res);
        assert!(res.is_ok());

        let (header, _) = res.unwrap();
        assert_eq!(header.audio_format, AudioFormat::PCMLinear);
        assert_eq!(header.channels, 1);
        assert_eq!(header.sample_rate, 44100);
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.bytes_per_block, 2);
    }

    #[test]
    fn test_header_f32() {
        let filename = Path::join(DATA_DIR.as_ref(), "single_sample_f32.wav");
        println!("Filename: {}", filename.display());

        let reader = File::open(filename).unwrap();
        let res = WavHeader::from_reader(reader);
        println!("{:#?}", res);
        assert!(res.is_ok());

        let (header, _) = res.unwrap();
    }

    #[test]
    fn test_iterator() {
        let filename = Path::join(DATA_DIR.as_ref(), "single_sample_u16.wav");
        println!("Filename: {}", filename.display());

        let reader = File::open(filename).unwrap();
        let res = super::WavFile::from_reader(reader);
        println!("{:#?}", res);
        assert!(res.is_ok());

        let iter_adt = res.unwrap().samples().unwrap();
        use crate::lowlevel::reader::SampleIteratorFormat::*;
        match iter_adt {
            I16(mut it) => {
                assert_eq!(it.next(), Some(0));
                assert!(it.next().is_none());
            }
            _ => panic!("Unexpected iterator format"),
        }
    }

    #[test]
    fn test_iterator_size() {
        let filename = Path::join(DATA_DIR.as_ref(), "single_sample_u16.wav");
        println!("Filename: {}", filename.display());

        let reader = File::open(filename).unwrap();
        let res = super::WavFile::from_reader(reader);
        println!("{:#?}", res);
        assert!(res.is_ok());

        let iter = res.unwrap().samples().unwrap();
        use crate::lowlevel::reader::SampleIteratorFormat::*;
        match iter {
            I16(mut it) => assert_eq!(1, it.len()),
            _ => panic!("Unexpected iterator format"),
        }
    }

    #[cfg(feature = "dasp")]
    #[test]
    fn test_iterator_sampling() {
        let filename = Path::join(DATA_DIR.as_ref(), "single_sample_u16.wav");
        println!("Filename: {}", filename.display());

        let reader = File::open(filename).unwrap();
        let res = super::WavFile::from_reader(reader);
        println!("{:#?}", res);
        assert!(res.is_ok());

        let mut iter = res.iter().sampled::<f32>();
        assert_eq!(iter.next(), Some(0.0));
        assert!(iter.next().is_none());
    }
}