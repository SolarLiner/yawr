use std::any::Any;
use std::io::{Write, Seek, SeekFrom};
use std::iter::FromIterator;
use std::marker::PhantomData;

use crate::lowlevel::WavFile;
use crate::sample::NumIO;
use byteorder::{WriteBytesExt, LittleEndian};

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("I/O Error: {0}")]
    IOError(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct WavWriter<T, W: Seek + Write> {
    file: WavFile<W>,
    data_size: u32,
    __phantom: PhantomData<T>,
}

impl<T, W: Seek + Write> Drop for WavWriter<T, W> {
    fn drop(&mut self) {
        self.patch_file();
    }
}

impl<T: NumIO, W: Seek + Write> WavWriter<T, W> {
    pub(crate) fn from_file(mut file: WavFile<W>) -> Self {
        assert_eq!(file.header.bits_per_sample as usize, 8 * std::mem::size_of::<T>());
        file.header.write(&mut file.data);
        Self {
            file,
            data_size: 0,
            __phantom: PhantomData,
        }
    }

    pub fn write_sample(&mut self, sample: T) -> Result<(), WriteError> {
        sample.write(&mut self.file.data)?;
        self.data_size += 1;
        Ok(())
    }

    pub fn write_iter<I: Iterator<Item=T>>(&mut self, iter: I) -> Result<(), WriteError> {
        for sample in iter {
            self.write_sample(sample)?;
        }
        Ok(())
    }
}

impl<T, W: Seek + Write> WavWriter<T, W> {
    fn patch_file(&mut self) -> Result<(), WriteError> {
        let file_size = self.data_size + 40; // Data size + header size - 4 bytes (position of the file size attribute)
        let data = &mut self.file.data;
        data.seek(SeekFrom::Start(4));
        data.write_u32::<LittleEndian>(file_size);

        data.seek(SeekFrom::Start(40));
        data.write_u32::<LittleEndian>(self.data_size);
        data.seek(SeekFrom::End(0));
        Ok(())
    }
}

impl<T, W: Clone + Seek + Write> WavWriter<T, W> {
    pub fn into_inner(mut self) -> Result<W, WriteError> {
        self.patch_file()?;
        Ok(self.file.data.clone())
    }
}