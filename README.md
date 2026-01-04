## redb-cli

A CLI tool to read/modify [redb](https://github.com/cberner/redb) database files.

Common key/value types are recognized by default (String, u64, Option, tuples,
etc.), unknown types are presumed to be raw strings.

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
  -l, --list     List tables and types
  -c, --create   Create database file and table
  -r, --remove   Remove key
  -d, --delete   Delete table
  -f, --force    Force update
  -j, --json     Output JSON
      --stats    Show table stats
      --check    Check integrity
      --compact  Compact database
  -h, --help     Print help
  -V, --version  Print version
```

```sh
$ redb-cli -l redb.db
users: u64 -> User
compound: (u64,i32) -> Log

$ redb-cli -c redb.db strings
Creating table "strings"

$ redb-cli -l redb.db
strings: String -> String
users: u64 -> User
compound: (u64,i32) -> Log

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

Redb types aren't designed with dynamic decoding in mind. Inside the db, types
are only identified by a name and a special flag denoting whether the type is
"internally" derived. As opposed to using a well-defined schema that can be
included in table type definition and used by tools like redb-cli to
dynamically decode data and present it to a user in a human-readable way.

The current approach is to create a predefined list covering combinations of
basic Rust types, encoder/decoder code is generated for each one individually.
The downside is long compilation time and large binary size, as well as that
not all supported types are covered. Dynamically doing encoding/decoding based
on type name could be an improvement.
