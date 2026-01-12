use eyre::{Result, bail};
use redb::{Database, ReadOnlyDatabase, ReadTransaction, ReadableDatabase, WriteTransaction};
use std::collections::BTreeMap;

pub mod cli;
pub mod parser;
pub mod transcode;
pub mod utils;

pub mod print;
pub mod process_multimap;
pub mod process {
    // Code for processing normal tables is derived from src/process_multimap.rs in build.rs
    include!(concat!(env!("OUT_DIR"), "/process.rs"));
}

#[cfg(test)]
mod test_ordering;
#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_structs;

pub const WARNING: &str = "\x1b[1m\x1b[33mwarning\x1b(B\x1b[m:";

pub enum DB {
    R(ReadOnlyDatabase),
    RW(Database),
}

impl DB {
    pub fn begin_read(&self) -> Result<ReadTransaction> {
        Ok(match self {
            DB::R(db) => db.begin_read(),
            DB::RW(db) => db.begin_read(),
        }?)
    }
    pub fn begin_write(&self) -> Result<WriteTransaction> {
        match self {
            DB::R(_) => bail!("Open in read-only mode"),
            DB::RW(db) => Ok(db.begin_write()?),
        }
    }
    pub fn check_integrity(&mut self) -> Result<bool> {
        match self {
            DB::R(_) => bail!("Open in read-only mode"),
            DB::RW(db) => Ok(db.check_integrity()?),
        }
    }
    pub fn compact(&mut self) -> Result<bool> {
        match self {
            DB::R(_) => bail!("Open in read-only mode"),
            DB::RW(db) => Ok(db.compact()?),
        }
    }
}

pub struct KVType<T> {
    pub k_ty: T,
    pub v_ty: T,
    pub is_multi: bool,
}

#[derive(Default)]
pub struct Data {
    pub stats: BTreeMap<String, BTreeMap<String, u64>>,
    pub list: BTreeMap<String, Vec<String>>,
    pub out: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    pub types: BTreeMap<String, KVType<&'static String>>,
}
