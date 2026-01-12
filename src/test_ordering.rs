#[test]
fn test_ordering() {
    macro_rules! test {
        (let $v:ident: $t:ty = $l:expr, $r:expr;) => {
            println!();
            println!("{}", stringify!(let $v: $t = $l , $r;));

            let l: $t = $l;
            let r: $t = $r;
            let ty = <$t as redb::Value>::type_name();
            let width = <$t as redb::Value>::fixed_width();
            let l_buf = <$t as redb::Value>::as_bytes(&l);
            let r_buf = <$t as redb::Value>::as_bytes(&r);

            let tree = crate::parser::parse_tree(ty.name()).unwrap();
            assert_eq!(crate::parser::parse_size(tree.clone()).unwrap(), width);

            let l_buf: &[u8] = l_buf.as_ref();
            let r_buf: &[u8] = r_buf.as_ref();

            let l_serde = serde_json::to_string_pretty(&l).unwrap();
            let r_serde = serde_json::to_string_pretty(&r).unwrap();

            let l_parsed_value = crate::parser::parse(tree.clone(), &mut l_buf.as_ref()).unwrap();
            let l_parsed = serde_json::to_string_pretty(&l_parsed_value).unwrap();

            let r_parsed_value = crate::parser::parse(tree.clone(), &mut r_buf.as_ref()).unwrap();
            let r_parsed = serde_json::to_string_pretty(&r_parsed_value).unwrap();

            println!("{l_serde}\n{l_parsed}");
            assert_eq!(l_serde, l_parsed);

            println!("{r_serde}\n{r_parsed}");
            assert_eq!(r_serde, r_parsed);

            let mut l_encoded = Vec::new();
            crate::parser::encode(tree.clone(), &l_parsed_value, &mut l_encoded).unwrap();
            crate::utils::dump_assert_eq(l_buf, &l_encoded);

            let mut r_encoded = Vec::new();
            crate::parser::encode(tree.clone(), &r_parsed_value, &mut r_encoded).unwrap();
            crate::utils::dump_assert_eq(r_buf, &r_encoded);

            assert_eq!(
                crate::parser::ordering(&l_parsed_value, &r_parsed_value).unwrap(),
                <$t as redb::Key>::compare(l_buf, r_buf),
            );
        };
        ($(let $v:ident: $t:ty = $l:expr, $r:expr;)*) => {
            $( test!( let $v: $t = $l, $r; ); )*
        };
    }

    // Integer comparisons
    test! {
        // i8
        let val: i8 = 0, 2;
        let val: i8 = -128, -127;
        let val: i8 = 127, 126;
        let val: i8 = 0, -1;
        let val: i8 = 100, 100;
        let val: i8 = -100, 100;

        // i16
        let val: i16 = -32768, -32767;
        let val: i16 = 32767, 32766;
        let val: i16 = 0, 1000;
        let val: i16 = -1000, 1000;
        let val: i16 = 0, -1000;

        // i32
        let val: i32 = -2147483648, -2147483647;
        let val: i32 = 2147483647, 2147483646;
        let val: i32 = 0, 1000000;
        let val: i32 = -1000000, 1000000;
        let val: i32 = 0, -1000000;

        // i64
        let val: i64 = -9223372036854775808, -9223372036854775807;
        let val: i64 = 9223372036854775807, 9223372036854775806;
        let val: i64 = 0, 1000000000000;
        let val: i64 = -1000000000000, 1000000000000;

        // i128
        let val: i128 = -170141183460469231731687303715884105728, -170141183460469231731687303715884105727;
        let val: i128 = 170141183460469231731687303715884105727, 170141183460469231731687303715884105726;

        // u8
        let val: u8 = 0, 1;
        let val: u8 = 255, 254;
        let val: u8 = 100, 100;
        let val: u8 = 0, 255;

        // u16
        let val: u16 = 0, 65535;
        let val: u16 = 65535, 65534;
        let val: u16 = 1000, 2000;

        // u32
        let val: u32 = 0, 4294967295;
        let val: u32 = 4294967295, 4294967294;
        let val: u32 = 1000000, 2000000;

        // u64
        let val: u64 = 0, 18446744073709551615;
        let val: u64 = 18446744073709551615, 18446744073709551614;
    }

    // Boolean comparisons
    test! {
        let val: bool = false, true;
        let val: bool = true, false;
        let val: bool = false, false;
        let val: bool = true, true;
    }

    // Character comparisons
    test! {
        let val: char = 'a', 'b';
        let val: char = 'a', 'A';
        let val: char = 'z', 'a';
        let val: char = '0', '9';
        let val: char = ' ', 'a';
        let val: char = '🦀', 'a';
        let val: char = '😀', '😃';
        let val: char = '\0', 'a';
        let val: char = '\n', ' ';
        let val: char = char::MAX, 'a';
        let val: char = 'α', 'β';
    }

    // String/str comparisons
    test! {
        // &str comparisons
        let val: &str = "a", "b";
        let val: &str = "abc", "abd";
        let val: &str = "abc", "abcd";
        let val: &str = "", "a";
        let val: &str = "hello", "world";
        let val: &str = "Hello", "hello";
        let val: &str = "123", "124";
        let val: &str = "🦀", "🐢";
        let val: &str = "αβγ", "αβδ";
        let val: &str = "a\nb", "a\rb";

        // String comparisons
        let val: String = String::from("a"), String::from("b");
        let val: String = String::from(""), String::from("a");
        let val: String = String::from("hello"), String::from("world");
        let val: String = String::from("abc"), String::from("abc");
    }

    // Tuple comparisons
    test! {
        // 2-tuples
        let val: (i32, i32) = (1, 2), (1, 3);
        let val: (i32, i32) = (1, 2), (2, 1);
        let val: (i32, i32) = (1, 2), (1, 2);
        let val: (i32, &str) = (1, "a"), (1, "b");
        let val: (i32, &str) = (2, "a"), (1, "z");

        // 3-tuples
        let val: (i32, i32, i32) = (1, 2, 3), (1, 2, 4);
        let val: (i32, i32, i32) = (1, 2, 3), (1, 3, 1);

        // Mixed type tuples
        let val: (bool, i32, &str) = (true, 1, "a"), (false, 100, "z");
        let val: (bool, i32, &str) = (true, 1, "a"), (true, 1, "b");

        // Nested tuples
        let val: ((i32, i32), i32) = ((1, 2), 3), ((1, 2), 4);
        let val: ((i32, i32), i32) = ((1, 2), 3), ((1, 3), 1);
    }

    // Option comparisons
    test! {
        // Simple Options
        let val: Option<i32> = Some(1), Some(2);
        let val: Option<i32> = Some(1), None;
        let val: Option<i32> = None, Some(1);
        let val: Option<i32> = None, None;
        let val: Option<i32> = Some(1), Some(1);

        // Option with other types
        let val: Option<&str> = Some("a"), Some("b");
        let val: Option<&str> = Some("a"), None;
        let val: Option<bool> = Some(true), Some(false);

        // Nested Options
        // This edge case we can't cover as long as we use [`serde_json::Value`]
        // let val: Option<Option<i32>> = Some(Some(1)), Some(None);
        // let val: Option<Option<i32>> = Some(None), None;
        // let val: Option<Option<i32>> = Some(Some(1)), Some(Some(2));
    }

    // Array comparisons
    test! {
        let val: [i32; 3] = [1, 2, 3], [1, 2, 4];
        let val: [i32; 3] = [1, 2, 3], [1, 3, 2];
        let val: [i32; 0] = [], [];
        let val: [i32; 2] = [1, 2], [1, 2];
        let val: [char; 2] = ['a', 'b'], ['a', 'c'];
    }

    // Slice comparisons
    test! {
        let val: &[u8] = b"abc", b"abd";
        let val: &[u8] = &[1, 2, 3], &[1, 2];
    }

    // Byte comparisons
    test! {
        let val: u8 = b'a', b'b';
        let val: u8 = 0, 255;
        let val: &[u8] = &[0, 1, 2], &[0, 1, 3];
    }

    // Edge cases and special comparisons
    test! {
        // Comparing with min/max
        let val: i8 = i8::MIN, i8::MAX;
        let val: u8 = u8::MIN, u8::MAX;

        // Comparing with min/max
        let val: i64 = i64::MIN, i64::MAX;
        let val: u64 = u64::MIN, u64::MAX;

        // Comparing with min/max
        let val: i128 = i128::MIN, i128::MAX;
        let val: u128 = u128::MIN, u128::MAX;
    }

    // Complex nested type comparisons
    test! {
        // Nested tuples and Options
        let val: (Option<i32>, (i32, &str)) = (Some(1), (2, "a")), (None, (2, "a"));
        let val: (Option<i32>, (i32, &str)) = (Some(2), (2, "a")), (None, (2, "a"));
        let val: (Option<i32>, (i32, &str)) = (Some(1), (2, "a")), (Some(1), (2, "b"));
        let val: (Option<i32>, (i32, &str)) = (Some(2), (2, "a")), (Some(1), (2, "b"));
    }
}
