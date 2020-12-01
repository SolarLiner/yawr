use crate::lowlevel::WavFile;
use std::marker::PhantomData;
use crate::sample::NumIO;
use std::io::Read;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Unexpected {0}, expecing magic number 'RIFF'")]
    ExpectedRIFF(String),
    #[error("Unexpected {0}, expecting magic number 'WAVE'")]
    ExpectedWAVE(String),
    #[error("Unexpected {0}, expecting magic number 'fmt '")]
    ExpectedFmt(String),
    #[error("Unexpected {0}, expecting magic number 'data'")]
    ExpectedData(String),
    #[error("I/O Error: {0}")]
    IOError(#[from] std::io::Error),
}

pub struct WavSampleIterator<T, R> {
    pub(crate) file: WavFile<R>,
    pub(crate) __type: PhantomData<T>,
}

impl<T: NumIO, R: Read> Iterator for WavSampleIterator<T, R> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        T::read(&mut self.file.data).ok()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let s = self.file.len();
        (s, Some(s))
    }
}

impl<T: NumIO, R: Read> ExactSizeIterator for WavSampleIterator<T, R> {
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

pub enum SampleIteratorFormat<R> {
    U8(WavSampleIterator<u8, R>),
    I16(WavSampleIterator<i16, R>),
    #[cfg(feature = "dasp")]
    I24(WavSampleIterator<dasp_sample::I24, R>),
    I32(WavSampleIterator<i32, R>),
    #[cfg(feature = "dasp")]
    I48(WavSampleIterator<dasp_sample::I48, R>),
    I64(WavSampleIterator<i64, R>),
    F32(WavSampleIterator<f32, R>),
    F64(WavSampleIterator<f64, R>),
}

#[cfg(feature = "dasp")]
impl<R: Read> SampleIteratorFormat<R> {
    pub fn sampled<T>(self) -> impl Iterator<Item=T>
        where T: dasp_sample::Sample +
        dasp_sample::FromSample<u8> +
        dasp_sample::FromSample<i16> +
        dasp_sample::FromSample<dasp_sample::I24> +
        dasp_sample::FromSample<i32> +
        dasp_sample::FromSample<i64> +
        dasp_sample::FromSample<f32> +
        dasp_sample::FromSample<f64> {
        use SampleIteratorFormat::*;
        match self {
            U8(it) => it.map(dasp_sample::FromSample::from_sample_),
            I16(it) => it.map(dasp_sample::FromSample::from_sample_),
            I24(it) => it.map(dasp_sample::FromSample::from_sample_),
            I32(it) => it.map(dasp_sample::FromSample::from_sample_),
            I48(it) => it.map(dasp_sample::FromSample::from_sample_),
            I64(it) => it.map(dasp_sample::FromSample::from_sample_),
            F32(it) => it.map(dasp_sample::FromSample::from_sample_),
            F64(it) => it.map(dasp_sample::FromSample::from_sample_),
        }
    }
}
