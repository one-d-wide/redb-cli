use eyre::Result;
use pest::iterators::Pair;
use redb::{TypeName, Value};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
};

use crate::parser::{Rule, encode, ordering, parse_from_tree};

std::thread_local! {
    pub static K_NAME: RefCell<TypeName> = RefCell::new(String::type_name());
    pub static K_WIDTH: Cell<Option<usize>> = const { Cell::new(None) };
    pub static K_TREE: RefCell<Option<Pair<'static, Rule>>> = RefCell::new(None);

    pub static V_NAME: RefCell<TypeName> = RefCell::new(String::type_name());
    pub static V_WIDTH: Cell<Option<usize>> = const { Cell::new(None) };
    pub static V_TREE: RefCell<Option<Pair<'static, Rule>>> = RefCell::new(None);
}

pub fn val_to_string(ty: &'static str, val: serde_json::Value) -> String {
    if matches!(ty, "&str" | "String") {
        match val {
            serde_json::Value::String(res) => res,
            _ => panic!("Invalid value type"),
        }
    } else {
        serde_json::to_string(&val).unwrap()
    }
}

pub fn string_to_val(ty: &'static str, val: &str) -> Result<serde_json::Value> {
    if matches!(ty, "&str" | "String") {
        Ok(serde_json::Value::String(val.to_string()))
    } else {
        Ok(serde_json::from_str(val)?)
    }
}

#[derive(Debug)]
pub struct K;

#[derive(Debug)]
pub struct V;

impl redb::Key for K {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        let ty = K_TREE.with_borrow(|t| t.clone().unwrap());
        let l = parse_from_tree(ty.clone(), data1).unwrap();
        let r = parse_from_tree(ty.clone(), data2).unwrap();
        ordering(&l, &r).unwrap()
    }
}

impl redb::Key for V {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        let ty = V_TREE.with_borrow(|t| t.clone().unwrap());
        let l = parse_from_tree(ty.clone(), data1).unwrap();
        let r = parse_from_tree(ty.clone(), data2).unwrap();
        ordering(&l, &r).unwrap()
    }
}

impl redb::Value for K {
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    type SelfType<'a>
        = serde_json::Value
    where
        Self: 'a;

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let ty = K_TREE.with_borrow(|t| t.clone().unwrap());
        let mut buf = Vec::new();
        encode(ty, value, &mut buf).unwrap();
        buf
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let ty = K_TREE.with_borrow(|t| t.clone().unwrap());
        parse_from_tree(ty, data).unwrap()
    }

    fn fixed_width() -> Option<usize> {
        K_WIDTH.get()
    }

    fn type_name() -> redb::TypeName {
        K_NAME.with_borrow(|n| n.clone())
    }
}

impl redb::Value for V {
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    type SelfType<'a>
        = serde_json::Value
    where
        Self: 'a;

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        let ty = V_TREE.with_borrow(|t| t.clone().unwrap());
        let mut buf = Vec::new();
        encode(ty, value, &mut buf).unwrap();
        buf
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let ty = V_TREE.with_borrow(|t| t.clone().unwrap());
        parse_from_tree(ty, data).unwrap()
    }

    fn fixed_width() -> Option<usize> {
        V_WIDTH.get()
    }

    fn type_name() -> redb::TypeName {
        V_NAME.with_borrow(|n| n.clone())
    }
}
