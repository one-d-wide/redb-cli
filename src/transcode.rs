use std::{
    cell::{Cell, RefCell},
    cmp::{Ord, Ordering},
};

use redb::{TypeName, Value};

std::thread_local! {
    pub static K_NAME: RefCell<TypeName> = RefCell::new(String::type_name());
    pub static K_WIDTH: Cell<Option<usize>> = const { Cell::new(None) };
    pub static K_PARAMS: Cell<Option<EncDe>> = const { Cell::new(None) };

    pub static V_NAME: RefCell<TypeName> = RefCell::new(String::type_name());
    pub static V_WIDTH: Cell<Option<usize>> = const { Cell::new(None) };
    pub static V_PARAMS: Cell<Option<EncDe>> = const { Cell::new(None) };
}

#[derive(Debug)]
pub struct K;

#[derive(Debug)]
pub struct V;

impl redb::Key for K {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        (K_PARAMS.get().unwrap().ord)(data1, data2)
    }
}

impl redb::Value for K {
    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    type SelfType<'a>
        = String
    where
        Self: 'a;

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        (K_PARAMS.get().unwrap().enc)(value)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        (K_PARAMS.get().unwrap().de)(data)
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
        = String
    where
        Self: 'a;

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        (V_PARAMS.get().unwrap().enc)(value)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        (V_PARAMS.get().unwrap().de)(data)
    }

    fn fixed_width() -> Option<usize> {
        V_WIDTH.get()
    }

    fn type_name() -> redb::TypeName {
        V_NAME.with_borrow(|n| n.clone())
    }
}

#[derive(Clone, Copy)]
pub struct EncDe {
    de: fn(data: &[u8]) -> String,
    enc: fn(value: &str) -> Vec<u8>,
    ord: fn(l: &[u8], r: &[u8]) -> Ordering,
}

impl EncDe {
    pub fn new_for_slices() -> Self {
        EncDe {
            de: |t: &[u8]| String::from_utf8_lossy(t).to_string(),
            enc: |t: &str| t.as_bytes().to_vec(),
            ord: Ord::cmp,
        }
    }
}

pub fn enc_de(name: &TypeName) -> Option<EncDe> {
    macro_rules! via_fallback {
        ($($t:ty,)* ) => {
            $(
                if name == &<$t>::type_name() {
                    return Some(EncDe::new_for_slices());
                }
            )*
        };
    }

    macro_rules! via_json0 {
        ($t:ty) => {
            if name == &<$t>::type_name() {
                return Some(EncDe {
                    de: |t: &[u8]| serde_json::to_string(&<$t>::from_bytes(t)).unwrap(),
                    enc: |t: &str| <$t>::as_bytes(&serde_json::from_str::<$t>(t).unwrap()).into(),
                    ord: |l: &[u8], r: &[u8]| Ord::cmp(&<$t>::from_bytes(l), &<$t>::from_bytes(r)),
                });
            }
        };
    }

    macro_rules! via_json1 {
        ($($t:ty,)* ) => {
            $(
                via_json0!(Option<$t>);
                via_json0!(Vec<$t>);
                via_json0!(($t,));
            )*
        };
    }

    macro_rules! via_json2 {
        ($t1:ty, $($t2:ty,)*) => {
            $(
                via_json0!(Option<($t1, $t2)>);
                via_json0!(($t1, $t2));
            )*
        };
    }

    macro_rules! via_json {
        ($($t:ty,)*) => {
            $(via_json0!($t);)*
            via_json1!($($t,)*);

            // Comment the lines below to speed up compilation

            $( via_json2!($t,
                u8, u16, u32, u64, u128,
                i8, i16, i32, i64, i128,
                &str, &[u8],
                (), bool, char,
                String,
            ); )*

        };
    }

    via_fallback! {
        &str, String,
    };

    via_json! {
        u8, u16, u32, u64, u128,
        i8, i16, i32, i64, i128,
        &str, &[u8],
        (), bool, char,
        String,
    };

    None
}
