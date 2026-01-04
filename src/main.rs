use clap::Parser;
use eyre::bail;
use redb::{
    ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableError,
    TableHandle, Value,
};
use std::{collections::BTreeMap, io::Write};

mod transcode;

use transcode::{EncDe, K, K_NAME, K_PARAMS, K_WIDTH, V, V_NAME, V_PARAMS, V_WIDTH};

pub const WARNING: &str = "\x1b[1m\x1b[33mwarning\x1b(B\x1b[m:";

/// A CLI tool to read/modify redb database files
#[derive(Parser, Debug, Clone)]
#[command(version)]
struct CliArgs {
    /// Database file
    #[arg(required = true)]
    file: String,

    /// Table name
    table: Option<String>,

    /// Key (raw string or JSON value)
    key: Option<String>,

    /// Value (raw string or JSON value)
    value: Option<String>,

    /// List tables and types
    #[arg(short, long, conflicts_with = "remove")]
    list: bool,

    /// Create database file and table
    #[arg(short, long)]
    create: bool,

    /// Remove key
    #[arg(short, long, requires = "key")]
    remove: bool,

    /// Delete table
    #[arg(short, long, requires = "table", conflicts_with = "key")]
    delete: bool,

    /// Force update
    #[arg(short, long)]
    force: bool,

    /// Output JSON
    #[arg(short, long)]
    json: bool,

    /// Show table stats
    #[arg(long, conflicts_with = "list")]
    stats: bool,

    /// Check integrity
    #[arg(long)]
    check: bool,

    /// Compact database
    #[arg(long)]
    compact: bool,
}

fn main() -> eyre::Result<()> {
    let args = CliArgs::parse();
    let mut db = if args.create {
        redb::Database::create(&args.file)?
    } else {
        redb::Database::open(&args.file)?
    };

    if args.check {
        db.check_integrity()?;
        return Ok(());
    }

    if args.compact {
        db.compact()?;
        return Ok(());
    }

    let mut w = db.begin_write()?;

    let tables = match &args.table {
        Some(t) => vec![t.clone()],
        None => w.list_tables()?.map(|t| t.name().to_string()).collect(),
    };

    let mut stats: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut list: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    for table_name in &tables {
        K_NAME.set(String::type_name());
        K_PARAMS.set(None);
        V_NAME.set(String::type_name());
        V_PARAMS.set(None);

        let table_def = TableDefinition::<K, V>::new(table_name);
        for _ in 0..5 {
            match db.begin_read()?.open_table(table_def) {
                Err(TableError::TableTypeMismatch { key, value, .. }) => {
                    K_NAME.set(key);
                    V_NAME.set(value)
                }
                Err(TableError::TypeDefinitionChanged { name, width, .. }) => {
                    if K_NAME.with_borrow(|n| n == &name) {
                        K_WIDTH.set(width);
                    }
                    if V_NAME.with_borrow(|n| n == &name) {
                        V_WIDTH.set(width);
                    }
                }
                Err(TableError::TableDoesNotExist(_)) if args.create => {
                    println!("Creating table {table_name:?}");
                    w.open_table(table_def)?;
                    w.commit()?;
                    w = db.begin_write()?;
                }
                Err(err) => return Err(err.into()),
                Ok(_) => break,
            }
        }

        {
            let k = K_NAME.with_borrow(|n| n.clone());
            let v = V_NAME.with_borrow(|n| n.clone());

            K_PARAMS.set(transcode::enc_de(&k));
            V_PARAMS.set(Some(
                transcode::enc_de(&v).unwrap_or(EncDe::new_for_slices()),
            ));

            if K_PARAMS.get().is_none() {
                K_PARAMS.set(Some(EncDe::new_for_slices()));

                println!(
                    "{WARNING} Key type {:?} of table {table_name:?} not recognized. Output may be incomplete.",
                    k.name(),
                );

                if !args.force && (args.value.is_some() || args.remove) {
                    bail!("Refusing to modify table with unrecognized key");
                }
            }
        }

        if args.list {
            let out = list.entry(table_name.clone()).or_default();
            let (k, v) = (K::type_name(), V::type_name());
            out.push(k.name().to_string());
            out.push(v.name().to_string());
            continue;
        }

        let out = out.entry(table_name.clone()).or_default();
        let mut table = w.open_table(table_def)?;

        if args.stats {
            let stat = table.stats()?;
            let stats = stats.entry(table_name.clone()).or_default();
            macro_rules! dump_stats {
                ($($stat:ident,)*) => {
                    $( stats.insert(stringify!($stat).into(), stat.$stat().into()); )*
                };
            }
            dump_stats!(
                tree_height,
                leaf_pages,
                branch_pages,
                stored_bytes,
                metadata_bytes,
                fragmented_bytes,
            );
            continue;
        }

        if args.delete {
            w.delete_table(table)?;
            w.commit()?;
            return Ok(());
        }

        match (&args.key, &args.value) {
            (None, None) => {
                for r in table.iter()? {
                    let (k, v) = r?;
                    let k = k.value();
                    let v = v.value();

                    out.insert(k, v);
                }
            }
            (Some(k), _) if args.remove => {
                if table.remove(k)?.is_none() {
                    bail!("No such key {k:?} in {table_name:?}");
                }
                drop(table);
                w.commit()?;
                return Ok(());
            }
            (Some(k), None) => {
                let v = table.get(k)?;
                let Some(v) = v else {
                    bail!("No such key {k:?} in {table_name:?}");
                };

                out.insert(k.clone(), v.value().to_string());
            }
            (Some(k), Some(v)) => {
                let old = table.insert(k, v.clone())?;
                if let Some(v) = old {
                    let v = v.value();

                    out.insert(k.clone(), v);
                }
            }
            _ => unreachable!(),
        }
    }

    w.commit()?;

    if args.json {
        let mut stdout = std::io::stdout().lock();
        match (&args.table, &args.key, &args.value) {
            _ if args.list => serde_json::to_writer_pretty(&mut stdout, &list)?,
            _ if args.stats => serde_json::to_writer_pretty(&mut stdout, &stats)?,
            (None, _, _) => serde_json::to_writer_pretty(&mut stdout, &out)?,
            (Some(t), None, _) => serde_json::to_writer_pretty(&mut stdout, &out[t])?,
            (Some(t), Some(k), None) => serde_json::to_writer_pretty(&mut stdout, &out[t][k])?,
            _ => {}
        }
        writeln!(stdout)?;
    } else {
        match (&args.table, &args.key, &args.value) {
            _ if args.list => {
                for (table, types) in &list {
                    println!("{table}: {} -> {}", types[0], types[1]);
                }
            }
            _ if args.stats => {
                for (table, out) in &stats {
                    println!();
                    println!("{table}:");
                    for (k, v) in out {
                        println!("{k}: {v}");
                    }
                }
            }
            (None, _, _) => {
                for (table, out) in &out {
                    println!();
                    println!("{table}:");
                    for (k, v) in out {
                        println!("{k}: {v}");
                    }
                }
            }
            (Some(t), None, _) => {
                for (k, v) in &out[t] {
                    println!("{k}: {v}");
                }
            }
            (Some(t), Some(k), None) => {
                println!("{}", &out[t][k]);
            }
            _ => {}
        }
    }

    Ok(())
}
