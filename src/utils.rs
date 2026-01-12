#[allow(unused)]
pub fn dump_hex(buf: &[u8]) {
    let mut len = 0;
    for chunk in buf.chunks(16) {
        print!("{len:04x?}: ");
        print!("{chunk:02x?} ");
        for b in chunk {
            if b.is_ascii() && !b.is_ascii_control() {
                print!("{}", char::from_u32(*b as u32).unwrap());
            } else {
                print!(".");
            }
        }
        println!();
        len += chunk.len();
    }
}

#[allow(unused)]
#[track_caller]
pub fn dump_assert_eq(left: &[u8], right: &[u8]) {
    if left.len() != right.len() {
        dump_hex(left);
        dump_hex(right);
        panic!("Length mismatched");
    }
    if let Some(pos) = left.iter().zip(right.iter()).position(|(l, r)| *l != *r) {
        println!();
        println!("Left:");
        dump_hex(left);
        println!();
        println!("Right:");
        dump_hex(right);
        panic!("Differ at byte {pos} (0x{pos:x?})");
    }
}

pub trait OrElseRes {
    type T;
    fn or_else_res<E, F>(self, f: F) -> std::result::Result<Self::T, E>
    where
        F: FnOnce() -> std::result::Result<Self::T, E>;
}

impl<T> OrElseRes for Option<T> {
    type T = T;
    fn or_else_res<E, F>(self, f: F) -> std::result::Result<Self::T, E>
    where
        F: FnOnce() -> std::result::Result<Self::T, E>,
    {
        match self {
            Some(val) => Ok(val),
            None => f(),
        }
    }
}
