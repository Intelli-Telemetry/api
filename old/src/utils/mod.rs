use crate::error::{AppResult, F1ServiceError};
pub(crate) use ids_generator::IdsGenerator;
pub(crate) use password_hash::*;
pub(crate) use ports::MachinePorts;
use std::mem;

mod bitset;
mod ids_generator;
mod password_hash;
mod ports;

#[inline(always)]
pub fn cast<T>(bytes: &[u8]) -> AppResult<&T> {
    if bytes.len() < mem::size_of::<T>() {
        Err(F1ServiceError::CastingError)?;
    }

    let alignment = mem::align_of::<T>();
    let ptr = bytes.as_ptr();

    if (ptr as usize) % alignment != 0 {
        Err(F1ServiceError::CastingError)?;
    }

    Ok(unsafe { &*(ptr.cast()) })
}
