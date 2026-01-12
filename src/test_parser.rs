#[macro_export]
macro_rules! test {
    (let $v:ident: $t:ty = $val:expr;) => {
        println!();
        println!("{}", stringify!(let $v: $t = $val;));

        let $v: $t = $val;
        let ty = <$t as redb::Value>::type_name();
        let width = <$t as redb::Value>::fixed_width();
        let buf = <$t as redb::Value>::as_bytes(&$v);
        crate::utils::dump_hex(buf.as_ref());

        let tree = crate::parser::parse_tree(ty.name()).unwrap();

        let serde = serde_json::to_value(&$v).unwrap();
        let serde = serde_json::to_string_pretty(&serde).unwrap();

        let parsed_value = crate::parser::parse(tree.clone(), &mut buf.as_ref()).unwrap();
        let parsed = serde_json::to_string_pretty(&parsed_value).unwrap();

        println!("{serde}\n{parsed}");
        assert_eq!(serde, parsed);

        assert_eq!(crate::parser::parse_size(tree.clone()).unwrap(), width);

        let mut encoded = Vec::new();
        crate::parser::encode_type(ty.name(), &parsed_value, &mut encoded).unwrap();
        crate::utils::dump_assert_eq(&buf.as_ref(), &encoded);
    };
    ($(let $v:ident: $t:ty = $val:expr;)*) => {
        $( test!( let $v: $t = $val; ); )*
    };
}

#[test]
fn test_parser() {
    test! {
        // Signed integers
        let val: i8 = -128;
        let val: i8 = 127;
        let val: i16 = -32768;
        let val: i16 = 32767;
        let val: i32 = -2147483648;
        let val: i32 = 2147483647;
        let val: i64 = -9223372036854775808;
        let val: i64 = 9223372036854775807;
        let val: i128 = -170141183460469231731687303715884105728;
        let val: i128 = 170141183460469231731687303715884105727;

        // Unsigned integers
        let val: u8 = 0;
        let val: u8 = 255;
        let val: u16 = 0;
        let val: u16 = 65535;
        let val: u32 = 0;
        let val: u32 = 4294967295;
        let val: u64 = 0;
        let val: u64 = 18446744073709551615;
        let val: u128 = 0;
        let val: u128 = 340282366920938463463374607431768211455;
    }

    // Tuples
    test! {
        let val: (i32, i64) = (42, 314);
        let val: (u8, bool, &str) = (255, true, "hello");
        let val: ((),) = ((),);
        let val: (i32, String, Vec<u8>) = (100, String::from("test"), vec![1, 2, 3]);
        let val: (Option<i32>, Option<i32>) = (Some(42), None);
        let val: (Vec<i32>, Vec<i32>) = (vec![], vec![1, 2, 3]);
        let val: (u8, u8, u8, u8) = (0, 1, 2, 3);
    }

    // Options
    test! {
        let val: Option<i32> = Some(321);
        let val: Option<i32> = None;
        let val: Option<String> = Some(String::from("hello"));
        let val: Option<String> = None;
        let val: Option<Vec<i32>> = Some(vec![1, 2, 3]);
        let val: Option<Vec<i32>> = None;
        let val: Option<Option<i32>> = Some(Some(42));
        // Nested option type can't be fully represented as long as we use [`serde_json::Value`]
        // let val: Option<Option<i32>> = Some(None);
        let val: Option<Option<i32>> = None;
        let val: Option<&str> = Some("world");
        let val: Option<&str> = None;
    }

    // Vectors
    test! {
        let val: Vec<i32> = vec![];
        let val: Vec<i32> = vec![1];
        let val: Vec<i32> = vec![1, 2, 3, 4, 5];
        let val: Vec<Vec<i32>> = vec![vec![1, 2], vec![3, 4]];
        let val: Vec<Vec<i32>> = vec![];
        let val: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
        let val: Vec<String> = vec![String::from("a"), String::from("b")];
        let val: Vec<&str> = vec!["a", "b", "c"];
        let val: Vec<u8> = vec![0, 255, 128];
        let val: Vec<bool> = vec![true, false, true];
    }

    // &str slices
    test! {
        let val: &str = "";
        let val: &str = "hello";
        let val: &str = "world!";
        let val: &str = "🦀";
        let val: &str = "with\nnewline";
        let val: &str = "with\ttab";
        let val: &str = "with spaces";
        let val: &str = "special chars: !@#$%^&*()";
        let val: &str = r#"raw string with "quotes""#;
    }

    // &[u8] slices
    test! {
        let val: &[u8] = &[];
        let val: &[u8] = &[0];
        let val: &[u8] = &[255];
        let val: &[u8] = &[1, 2, 3, 4, 5];
        let val: &[u8] = &[0, 255, 128, 64];
        let val: &[u8] = "hello".as_bytes();
        let val: &[u8] = &[b'a', b'b', b'c'];
        let val: &[u8] = &[0u8; 10];
        let val: &[u8] = &[255; 5];
    }

    // Strings
    test! {
        let val: String = String::new();
        let val: String = String::from("");
        let val: String = String::from("hello");
        let val: String = String::from("🦀 Rust 🦀");
        let val: String = String::from("multi\nline\nstring");
        let val: String = String::from("with\ttabs");
        let val: String = String::from("with spaces and punctuation!");
        let val: String = "to_string".to_string();
        let val: String = format!("formatted: {}", 42);
    }

    // Mixed complex types
    test! {
        // Nested combinations
        let val: Vec<Option<(i32, String)>> = vec![Some((1, String::from("a"))), None, Some((2, String::from("b")))];
        let val: Option<Vec<&str>> = Some(vec!["a", "b", "c"]);
        let val: Option<Vec<&str>> = None;
        let val: (&str, Vec<u8>, Option<i32>) = ("test", vec![1, 2, 3], Some(42));

        // Bytes in different forms
        let val: Vec<u8> = vec![72, 101, 108, 108, 111]; // "Hello" in ASCII
        let val: &[u8] = b"Hello World";
        let val: String = String::from_utf8(vec![72, 101, 108, 108, 111]).unwrap();

        // Edge cases
        let val: (Vec<i32>, Option<Vec<i32>>, &str) = (vec![], None, "");
        let val: (Option<Vec<&str>>, Option<Vec<&str>>) = (Some(vec![]), None);

        // Maximum/minimum values in containers
        let val: Vec<i32> = vec![i32::MIN, 0, i32::MAX];
        let val: Option<u64> = Some(u64::MAX);
        let val: Option<u64> = Some(u64::MIN);

        // Empty collections
        let val: Vec<String> = vec![];
        let val: Vec<Vec<i32>> = vec![];
        let val: Vec<Option<Vec<i32>>> = vec![];

        // Single-element cases
        let val: Vec<i32> = vec![42];
        let val: Option<Vec<i32>> = Some(vec![42]);
        let val: (i32,) = (42,);
        let val: &str = "x";
    }

    // Additional boundary cases
    test! {
        // Large collections
        let val: Vec<i32> = (0..1000).collect();
        let val: String = "a".repeat(1000);

        // Unicode edge cases
        let val: &str = "é";
        let val: &str = "😀";
        let val: &str = "a̐";

        // Null bytes
        let val: Vec<u8> = vec![0, 1, 0, 2, 0];
        let val: &[u8] = &[0, b'a', 0, b'b', 0];

        // Boolean vectors
        let val: Vec<bool> = vec![true, false, true, false, true];
        let val: Option<Vec<bool>> = Some(vec![true, false]);

        // Mixed with references
        let val: (&str, &[u8]) = ("test", &[1, 2, 3]);
        let val: Vec<&str> = vec!["a", "bc", "def"];
    }
}
