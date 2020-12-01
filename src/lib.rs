#[macro_use]
extern crate thiserror;

use std::io::{Seek, Write};
use std::marker::PhantomData;

use crate::lowlevel::writer::{WavWriter, WriteError};
use crate::lowlevel::AudioFormat;
use crate::sample::NumIO;

#[cfg(test)]
const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_data");

pub mod lowlevel;
pub mod sample;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct WavFileDesc<T> {
    pub channels: u16,
    pub sample_rate: u32,
    pub length: usize,
    __phantom: PhantomData<T>,
}

impl<T> From<lowlevel::WavHeader> for WavFileDesc<T> {
    fn from(h: lowlevel::WavHeader) -> Self {
        assert_eq!(h.bits_per_sample as usize, std::mem::size_of::<T>() * 8);
        Self {
            channels: h.channels,
            sample_rate: h.sample_rate,
            length: (h.data_size / h.bytes_per_block as u32) as usize,
            __phantom: PhantomData,
        }
    }
}

impl<T: 'static> From<WavFileDesc<T>> for lowlevel::WavHeader {
    fn from(desc: WavFileDesc<T>) -> Self {
        let bytes_per_sample = std::mem::size_of::<T>();
        let bits_per_sample = (8 * bytes_per_sample) as u16;
        let bytes_per_block = (desc.channels as usize * bytes_per_sample) as u16;
        Self {
            file_size: (desc.length * bytes_per_sample + 44) as u32,
            channels: desc.channels,
            sample_rate: desc.sample_rate,
            bytes_per_sec: desc.sample_rate * bytes_per_sample as u32,
            bytes_per_block,
            bits_per_sample,
            audio_format: AudioFormat::from_type::<T>(),
            data_size: 0,
        }
    }
}

impl<T> WavFileDesc<T> {
    pub fn new(channels: u16, sample_rate: u32, length: usize) -> Self {
        Self {
            channels,
            sample_rate,
            length,
            __phantom: PhantomData,
        }
    }

    pub fn empty(channels: u16, sample_rate: u32) -> Self {
        Self::new(channels, sample_rate, 0)
    }
}

pub trait IteratorExt: Iterator {
    fn write<W: Seek + Write>(
        self,
        writer: &mut lowlevel::writer::WavWriter<Self::Item, W>,
    ) -> Result<(), lowlevel::writer::WriteError>;
}

impl<T: NumIO, I: Iterator<Item = T>> IteratorExt for I {
    fn write<W: Seek + Write>(
        self,
        writer: &mut WavWriter<Self::Item, W>,
    ) -> Result<(), WriteError> {
        writer.write_iter(self)
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use std::fs::File;
    use std::io::{Cursor, Seek, SeekFrom};
    use std::path::Path;

    use crate::lowlevel::writer::WavWriter;
    use crate::lowlevel::{WavFile, WavHeader};
    use crate::{IteratorExt, WavFileDesc};

    use super::DATA_DIR;

    #[test]
    fn test_write_iterator_ext() {
        let mut writer = Cursor::new(vec![]);
        let desc = WavFileDesc::<u8>::empty(1, 44100);
        let mut writer = WavWriter::from_file(WavFile::write(desc, writer));
        (0..44100)
            .map(|s| s as f64 / (44100.0 / (440.0 * 2.0 * PI)))
            .map(f64::sin)
            .map(|s| ((s + 1.0) * 127.5) as u8)
            .write(&mut writer);

        let mut data = writer.into_inner().unwrap();
        data.seek(SeekFrom::Start(0));
        let tmpfilename = std::env::temp_dir().join("temp_iterator_write.wav");
        std::io::copy(&mut data, &mut File::create(&tmpfilename).unwrap()).unwrap();
        println!("Generated WAV written to {}", tmpfilename.display());
    }
}
