use core::fmt::Write;

pub(crate) use ids_generator::{IdsGenerator, UsedIds};
pub(crate) use password_hash::*;
pub(crate) use ports::MachinePorts;

mod ids_generator;
mod password_hash;
mod ports;
mod time;

// Todo: Consider adding a trait to the String type to make this more idiomatic
pub fn write(query: &mut String, counter: &mut u8, field: &str) {
    if *counter > 1 {
        write!(query, ",").unwrap();
    }

    write!(query, " {} = ${}", field, counter).unwrap();
    *counter += 1;
}
