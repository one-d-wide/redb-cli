use eyre::{ContextCompat, Result, bail, eyre};
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;
use serde_json::{Number, Value};
use std::cmp::Ordering;
use thiserror::Error;

use crate::utils::OrElseRes;

pub type TakeResult<T> = std::result::Result<T, TakeError>;

#[derive(Error, Debug)]
pub enum TakeError {
    #[error("Off the data buffer")]
    OffBuffer,
}

#[track_caller]
pub fn take_n<'a>(data: &mut &'a [u8], len: usize) -> TakeResult<&'a [u8]> {
    let res;
    (res, *data) = data.split_at_checked(len).ok_or(TakeError::OffBuffer)?;
    Ok(res)
}

#[track_caller]
pub fn take_all<'a>(data: &mut &'a [u8]) -> &'a [u8] {
    let len = data.len();
    take_n(data, len).unwrap()
}

#[track_caller]
pub fn take<const N: usize>(data: &mut &[u8]) -> TakeResult<[u8; N]> {
    let res = take_n(data, N)?;
    Ok(res.try_into().unwrap())
}

#[track_caller]
pub fn take_u8(data: &mut &[u8]) -> TakeResult<u8> {
    Ok(take_n(data, 1)?[0])
}

#[track_caller]
pub fn take_u32_len(data: &mut &[u8]) -> TakeResult<usize> {
    Ok(u32::from_le_bytes(take(data)?) as usize)
}

#[track_caller]
pub fn take_varint(data: &mut &[u8]) -> TakeResult<usize> {
    Ok(match take_u8(data)? {
        n @ 0..=253 => n as usize,
        254 => u16::from_le_bytes(take(data)?) as usize,
        255 => u32::from_le_bytes(take(data)?) as usize,
    })
}

pub fn put_varint(len: usize, buf: &mut Vec<u8>) {
    if len < 254 {
        buf.push(len as u8);
    } else if len <= u16::MAX.into() {
        buf.push(254);
        buf.extend_from_slice(&u16::to_le_bytes(len as u16));
    } else {
        buf.push(255);
        buf.extend_from_slice(&u32::to_le_bytes(len as u32));
    }
}

#[derive(Parser)]
#[grammar = "src/grammar.pest"]
pub struct Grammar;

pub fn parse_from_tree(p: Pair<'_, Rule>, mut data: &[u8]) -> Result<serde_json::Value> {
    parse(p, &mut data)
}

pub fn parse_tree(ty: &str) -> Result<Pair<'_, Rule>> {
    Ok(Grammar::parse(Rule::FullType, ty)?.next().unwrap())
}

#[allow(unused)]
pub fn parse_type(ty: &str, mut data: &[u8]) -> Result<serde_json::Value> {
    parse(parse_tree(ty)?, &mut data)
}

#[allow(unused)]
pub fn encode_type(ty: &str, val: &serde_json::Value, buf: &mut Vec<u8>) -> Result<()> {
    encode(parse_tree(ty)?, val, buf)
}

pub fn encode(p: Pair<'_, Rule>, val: &serde_json::Value, buf: &mut Vec<u8>) -> Result<()> {
    let s = p.as_str();

    let err_type = || Err(eyre!("Unknown type {s:?}"));
    let err_val = || eyre!("Invalid value in {s:?}");
    let err_var_num = || Err(err_val());
    let err_var_u64 = || Err(err_val());
    let err_var_i128 = || Err(err_val());
    let err_var_u128 = || Err(err_val());
    let err_var_f64 = || Err(err_val());
    let err_var_arr = || Err(err_val());
    let err_var_str = || Err(err_val());
    let err_var_u8 = |err| eyre!("Error {s:?}: {err}");
    let err_num = |err| eyre!("Error {s:?}: {err}");

    match p.as_rule() {
        Rule::Bool => match val.as_bool().unwrap() {
            false => buf.push(0),
            true => buf.push(1),
        },
        Rule::Char => {
            let val = val.as_str().unwrap();
            if val.chars().count() != 1 {
                bail!("Char must be a single codepoint");
            }
            let val = <char as redb::Value>::as_bytes(&val.chars().next().unwrap());
            let val = val.as_ref();
            buf.extend_from_slice(val);
        }
        Rule::Int => {
            if let Some(bits) = s.strip_prefix("u") {
                let val = val
                    .as_number()
                    .or_else_res(err_var_num)?
                    .as_u128()
                    .or_else_res(err_var_u128)?;
                match bits {
                    "8" => buf.extend(u8::to_le_bytes(val.try_into().unwrap())),
                    "16" => buf.extend(u16::to_le_bytes(val.try_into().unwrap())),
                    "32" => buf.extend(u32::to_le_bytes(val.try_into().unwrap())),
                    "64" => buf.extend(u64::to_le_bytes(val.try_into().unwrap())),
                    "128" => buf.extend(u128::to_le_bytes(val.try_into().unwrap())),
                    _ => return err_type(),
                };
            } else if let Some(bits) = s.strip_prefix("i") {
                let val = val
                    .as_number()
                    .or_else_res(err_var_num)?
                    .as_i128()
                    .or_else_res(err_var_i128)?;
                match bits {
                    "8" => buf.extend(i8::to_le_bytes(val.try_into().unwrap())),
                    "16" => buf.extend(i16::to_le_bytes(val.try_into().unwrap())),
                    "32" => buf.extend(i32::to_le_bytes(val.try_into().unwrap())),
                    "64" => buf.extend(i64::to_le_bytes(val.try_into().unwrap())),
                    "128" => buf.extend(i128::to_le_bytes(val.try_into().unwrap())),
                    _ => return err_type(),
                };
            } else {
                unreachable!()
            }
        }
        Rule::Float => {
            let val = val
                .as_number()
                .or_else_res(err_var_num)?
                .as_f64()
                .or_else_res(err_var_f64)?;
            match s {
                "f32" => buf.extend(f32::to_le_bytes(val as f32)),
                "f64" => buf.extend(f64::to_le_bytes(val)),
                _ => return err_type(),
            }
        }
        Rule::String => {
            let val = val.as_str().or_else_res(err_var_str)?;
            buf.extend_from_slice(val.as_bytes());
        }
        Rule::Slice => {
            let val = val.as_array().or_else_res(err_var_arr)?;
            for v in val {
                let b = v
                    .as_number()
                    .or_else_res(err_var_num)?
                    .as_u64()
                    .or_else_res(err_var_u64)?;
                buf.push(b.try_into().map_err(err_var_u8)?);
            }
        }
        Rule::Array => {
            let mut iter = p.into_inner();
            let ty = iter.next().unwrap();
            let n = iter.next().unwrap();
            assert_eq!(n.as_rule(), Rule::Num);
            let n: usize = n.as_str().parse().map_err(err_num)?;

            let val = val.as_array().or_else_res(err_var_arr)?;
            if val.len() != n {
                bail!("Array length doesn't match {s:?}");
            }

            let ty_size = parse_size(ty.clone())?;

            let mut vec = Vec::new();
            for v in val {
                vec.clear();
                encode(ty.clone(), v, &mut vec)?;
                if ty_size.is_none() {
                    buf.extend_from_slice(u32::to_le_bytes(vec.len() as u32).as_slice());
                }
                buf.extend_from_slice(&vec);
            }
        }
        Rule::Option => {
            let ty = p.into_inner().next().unwrap();
            if val.is_null() {
                buf.push(0);
                if let Some(s) = parse_size(ty)? {
                    buf.extend(std::iter::repeat_n(0, s));
                }
            } else {
                buf.push(1);
                encode(ty, val, buf)?;
            }
        }
        Rule::Vec => {
            let ty = p.into_inner().next().unwrap();
            let val = val.as_array().or_else_res(err_var_arr)?;

            put_varint(val.len(), buf);

            let ty_size = parse_size(ty.clone())?;

            let mut vec = Vec::new();
            for i in 0..val.len() {
                vec.clear();
                encode(ty.clone(), &val[i], &mut vec)?;

                if ty_size.is_none() {
                    put_varint(vec.len(), buf);
                }
                buf.extend_from_slice(&vec);
            }
        }
        Rule::Tuple => {
            let val = if val.is_null() {
                &Vec::new()
            } else {
                val.as_array().or_else_res(err_var_arr)?
            };
            let iter = p.into_inner();
            assert_eq!(val.len(), iter.len());

            let mut iter = Iterator::zip(iter, val).peekable();

            let mut vec = Vec::new();
            while let Some((ty, val)) = iter.next() {
                let last_len = vec.len();
                encode(ty.clone(), val, &mut vec)?;

                let ty_size = parse_size(ty.clone())?;
                if ty_size.is_none() && iter.peek().is_some() {
                    put_varint(vec.len() - last_len, buf);
                }
            }
            buf.extend_from_slice(&vec);
        }
        Rule::Struct => {
            let mut iter = p.into_inner();
            let struct_name = iter.next().unwrap().as_str();

            let mut val = val.as_object().unwrap().clone();

            let mut vec = Vec::new();
            while iter.peek().is_some() {
                let name = iter.next().unwrap().as_str();
                let ty = iter.next().unwrap();

                let val = val
                    .remove(name)
                    .or_else(|| matches!(ty.as_rule(), Rule::Option).then_some(Value::Null))
                    .with_context(|| eyre!("Expected field {name:?} in struct {struct_name:?}"))?;

                let last_len = vec.len();
                encode(ty.clone(), &val, &mut vec)?;

                let ty_size = parse_size(ty.clone())?;
                if ty_size.is_none() && iter.peek().is_some() {
                    put_varint(vec.len() - last_len, buf);
                }
            }
            buf.extend_from_slice(&vec);

            if let Some((k, _)) = val.into_iter().next() {
                bail!("Struct {struct_name:?} has undefined field {k:?}");
            }
        }
        _ => return err_type(),
    }

    Ok(())
}

pub fn parse(p: Pair<'_, Rule>, data: &mut &[u8]) -> Result<serde_json::Value> {
    use serde_json::Value;

    let s = p.as_str();

    let err_type = || Err(eyre!("Unknown type {s:?}"));

    Ok(match p.as_rule() {
        Rule::Bool => match take_u8(data)? {
            0 => Value::Bool(false),
            1 => Value::Bool(true),
            x => bail!("Unknown bool state: {x:?}"),
        },
        Rule::Char => {
            let len = <char as redb::Value>::fixed_width().unwrap();
            Value::String(<char as redb::Value>::from_bytes(take_n(data, len)?).to_string())
        }
        Rule::Int => {
            if let Some(bits) = s.strip_prefix("u") {
                let num = match bits {
                    "8" => u8::from_le_bytes(take(data)?) as u128,
                    "16" => u16::from_le_bytes(take(data)?) as u128,
                    "32" => u32::from_le_bytes(take(data)?) as u128,
                    "64" => u64::from_le_bytes(take(data)?) as u128,
                    "128" => u128::from_le_bytes(take(data)?),
                    _ => return err_type(),
                };
                Value::Number(Number::from_u128(num).unwrap())
            } else if let Some(bits) = s.strip_prefix("i") {
                let num = match bits {
                    "8" => i8::from_le_bytes(take(data)?) as i128,
                    "16" => i16::from_le_bytes(take(data)?) as i128,
                    "32" => i32::from_le_bytes(take(data)?) as i128,
                    "64" => i64::from_le_bytes(take(data)?) as i128,
                    "128" => i128::from_le_bytes(take(data)?),
                    _ => return err_type(),
                };
                Value::Number(Number::from_i128(num).unwrap())
            } else {
                bail!("Unknown type {s:?}");
            }
        }
        Rule::Float => {
            let val = match s {
                "f32" => f32::from_le_bytes(take(data)?) as f64,
                "f64" => f64::from_le_bytes(take(data)?),
                _ => return err_type(),
            };
            Value::Number(Number::from_f64(val).unwrap())
        }
        Rule::String => Value::String(String::from_utf8(take_all(data).to_vec()).unwrap()),
        Rule::Slice => Value::Array(
            take_all(data)
                .iter()
                .map(|b| Value::Number(Number::from_u128(*b as u128).unwrap()))
                .collect(),
        ),
        Rule::Array => {
            let mut iter = p.into_inner();
            let ty = iter.next().unwrap();
            let n = iter.next().unwrap();

            assert_eq!(n.as_rule(), Rule::Num);
            let n: usize = n.as_str().parse().unwrap();

            let ty_size = parse_size(ty.clone())?;

            let mut vec = Vec::with_capacity(n);
            for _ in 0..n {
                let len = ty_size.or_else_res(|| take_u32_len(data))?;
                let val = parse(ty.clone(), &mut take_n(data, len)?)?;
                vec.push(val);
            }

            Value::Array(vec)
        }
        Rule::Vec => {
            let ty = p.into_inner().next().unwrap();
            let ty_size = parse_size(ty.clone())?;
            let n = take_varint(data)?;

            let mut vec = Vec::with_capacity(n);
            for _ in 0..n {
                let len = ty_size.or_else_res(|| take_varint(data))?;
                let val = parse(ty.clone(), &mut take_n(data, len)?)?;
                vec.push(val);
            }

            Value::Array(vec)
        }
        Rule::Option => {
            let ty = p.into_inner().next().unwrap();

            match take_u8(data)? {
                0 => {
                    if let Some(s) = parse_size(ty)? {
                        take_n(data, s)?;
                    }
                    Value::Null
                }
                1 => parse(ty, data)?,
                n => bail!("Invalid Option discriminant {n:?}"),
            }
        }
        Rule::Tuple => {
            let mut vec_ty = Vec::new();
            let mut iter = p.into_inner();
            while let Some(ty) = iter.next() {
                let mut ty_size = parse_size(ty.clone())?;
                if iter.peek().is_some() {
                    ty_size = Some(ty_size.or_else_res(|| take_varint(data))?);
                }
                vec_ty.push((ty, ty_size));
            }

            let mut vec = Vec::with_capacity(vec_ty.len());
            for (ty, ty_size) in vec_ty {
                let len = ty_size.unwrap_or_else(|| data.len());
                let val = parse(ty, &mut take_n(data, len)?)?;
                vec.push(val);
            }

            if vec.is_empty() {
                Value::Null
            } else {
                Value::Array(vec)
            }
        }
        Rule::Struct => {
            let mut iter = p.into_inner();
            let _struct_name = iter.next().unwrap();
            let mut vec = Vec::new();
            while iter.peek().is_some() {
                let name = iter.next().unwrap();
                let ty = iter.next().unwrap();

                let mut ty_size = parse_size(ty.clone())?;
                if ty_size.is_none() && iter.peek().is_some() {
                    ty_size = Some(take_varint(data)?);
                }

                vec.push((name.as_str(), ty, ty_size));
            }

            let mut map = serde_json::Map::with_capacity(vec.len());
            for (name, ty, ty_size) in vec {
                let len = ty_size.unwrap_or_else(|| data.len());
                let val = parse(ty, &mut take_n(data, len)?)?;
                map.insert(name.into(), val);
            }
            Value::Object(map)
        }
        _ => return err_type(),
    })
}

pub fn parse_size(p: Pair<'_, Rule>) -> Result<Option<usize>> {
    let s = p.as_str();

    let err_type = || Err(eyre!("Unknown type {s:?}"));

    Ok(match p.as_rule() {
        Rule::Bool => Some(1),
        Rule::Char => Some(<char as redb::Value>::fixed_width().unwrap()),
        Rule::Int => {
            let num: usize = s.strip_prefix(['u', 'i']).unwrap().parse().unwrap();
            match num {
                8 | 16 | 32 | 64 | 128 => Some(num / 8),
                _ => return err_type(),
            }
        }
        Rule::Float => match s {
            "f32" => Some(4),
            "f64" => Some(8),
            _ => return err_type(),
        },
        Rule::String => None,
        Rule::Slice => None,
        Rule::Array => {
            let mut iter = p.into_inner();
            let ty = iter.next().unwrap();
            let n = iter.next().unwrap();
            assert_eq!(n.as_rule(), Rule::Num);
            let n: usize = n.as_str().parse().unwrap();

            parse_size(ty)?.map(|x| x * n)
        }
        Rule::Vec => None,
        Rule::Option => {
            let ty = p.into_inner().next().unwrap();
            parse_size(ty)?.map(|x| 1 + x)
        }
        Rule::Tuple => {
            let mut sum = 0;
            for ty in p.into_inner() {
                match parse_size(ty)? {
                    Some(len) => sum += len,
                    None => return Ok(None),
                }
            }
            Some(sum)
        }
        Rule::Struct => {
            let mut sum = 0;
            let mut iter = p.into_inner();
            let _struct_name = iter.next().unwrap();
            while iter.peek().is_some() {
                let _field_name = iter.next().unwrap();
                let ty = iter.next().unwrap();
                match parse_size(ty)? {
                    Some(len) => sum += len,
                    None => return Ok(None),
                }
            }
            Some(sum)
        }
        _ => return err_type(),
    })
}

pub fn can_order(p: Pair<'_, Rule>) -> Result<bool> {
    let s = p.as_str();

    let err_type = || Err(eyre!("Unknown type {s:?}"));

    Ok(match p.as_rule() {
        Rule::Bool | Rule::Char | Rule::Int | Rule::String | Rule::Slice => true,
        Rule::Float => false,
        Rule::Array | Rule::Vec | Rule::Option => can_order(p.into_inner().next().unwrap())?,
        Rule::Tuple => {
            for ty in p.into_inner() {
                if !can_order(ty)? {
                    return Ok(false);
                }
            }
            true
        }
        Rule::Struct => {
            // Json objects don't preserve field order
            false
        }
        _ => return err_type(),
    })
}

pub fn ordering(l: &Value, r: &Value) -> Option<std::cmp::Ordering> {
    Some(match (l, r) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, t) if !t.is_null() => Ordering::Less,
        (t, Value::Null) if !t.is_null() => Ordering::Greater,

        (Value::Bool(l), Value::Bool(r)) => Ord::cmp(l, r),
        (Value::String(l), Value::String(r)) => Ord::cmp(l, r),

        (Value::Number(l), Value::Number(r)) => {
            if let Some((l, r)) = l.as_i128().zip(r.as_i128()) {
                Ord::cmp(&l, &r)
            } else if let Some((l, r)) = l.as_u128().zip(r.as_u128()) {
                Ord::cmp(&l, &r)
            } else {
                unreachable!()
            }
        }

        (Value::Array(l), Value::Array(r)) => {
            for (l, r) in Iterator::zip(l.iter(), r.iter()) {
                let c = ordering(l, r)?;
                if !c.is_eq() {
                    return Some(c);
                }
            }
            Ord::cmp(&l.len(), &r.len())
        }

        (Value::Object(_), Value::Object(_)) => return None,

        _ => unreachable!(),
    })
}
