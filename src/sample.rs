use std::io::{Read, Write};

pub trait NumIO: Sized {
    fn read<R: Read>(reader: &mut R) -> std::io::Result<Self>;
    fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

macro_rules! impl_numio {
    ($firsttype:ty, $($types:ty),*) => { impl_numio!($firsttype); $(impl_numio!($types);)* };
    ($type:ty) => {
        impl NumIO for $type {
            fn read<R: Read>(reader: &mut R) -> ::std::io::Result<Self> {
                let mut data = [0; ::std::mem::size_of::<$type>()];
                reader.read_exact(&mut data)?;
                Ok(<$type>::from_le_bytes(data))
            }
            fn write<W: Write>(&self, writer: &mut W) -> ::std::io::Result<()> {
                let data = self.to_le_bytes();
                writer.write_all(&data)?;
                Ok(())
            }
        }
    };
}

impl_numio!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
#[cfg(feature="dasp")]
impl_numio!(::dasp_sample::I24, ::dasp_sample::I48);