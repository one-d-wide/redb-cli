## redb-cli

A CLI tool to read/modify [redb](https://github.com/cberner/redb) database files.

All default types are supported (String, u64, Option, tuples, etc.), as well as
user types annotated with [redb-derive](https://docs.rs/redb-derive). Unknown
types are presumed to be raw strings.

## Usage

```sh
$ redb-cli --help
Usage: redb-cli [OPTIONS] <FILE> [TABLE] [KEY] [VALUE]

Arguments:
  <FILE>   Database file
  [TABLE]  Table name
  [KEY]    Key (raw string or JSON value)
  [VALUE]  Value (raw string or JSON value)

Options:
  -l, --list      List tables and types
  -c, --create    Create database file and table
  -r, --remove    Remove key
  -d, --delete    Delete table
  -m, --multimap  Open as multimap
  -j, --json      Output JSON
      --schema    Table schema, e.g. String -> String
      --ro        Open database read-only
      --stats     Show table stats
      --check     Check integrity
      --compact   Compact database
  -h, --help      Print help
  -V, --version   Print version
```

```sh
$ redb-cli -l redb.db
users: u64 -> String
compound: (u64,i32) -> Log { time: u64, line: String }

$ redb-cli -c redb.db strings
Creating table "strings"

$ redb-cli -l redb.db
strings: String -> String
users: u64 -> String
compound: (u64,i32) -> Log { time: u64, line: String }

$ redb-cli redb.db strings "hello" "world"

$ redb-cli -j redb.db strings
{
  "hello": "world",
}

$ redb-cli redb.db strings "hello"
world

$ redb-cli -r redb.db strings "hello"

$ redb-cli -j redb.db strings
{}

$ redb-cli -d redb.db strings
```

## Installation

```sh
$ cargo install --git https://github.com/one-d-wide/redb-cli
$ ~/.cargo/bin/redb-cli --help
```

## Limitations

We use a small rust-like [grammar](src/grammar.pest) to parse type names stored
in the table metadata, hence all types derived by redb should be supported.
Other encodings may be added later as well, preferably ones where schema-based
decoder already exists.
