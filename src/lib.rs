#[macro_use]
extern crate thiserror;

use std::io::Read;
use std::marker::PhantomData;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::sample::NumIO;
use lowlevel::WavFile;

pub(crate) mod sample;
pub mod lowlevel;
