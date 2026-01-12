use clap::Parser;
use eyre::Result;

use redb_cli::{DB, Data, cli::CliArgs, print, process, process_multimap};

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let mut db = if args.ro {
        DB::R(redb::ReadOnlyDatabase::open(&args.file)?)
    } else if args.create {
        DB::RW(redb::Database::create(&args.file)?)
    } else {
        DB::RW(redb::Database::open(&args.file)?)
    };

    if args.check {
        db.check_integrity()?;
        return Ok(());
    }

    if args.compact {
        db.compact()?;
        return Ok(());
    }

    let mut data = Data::default();

    if args.multimap {
        process_multimap::process_multimap(&args, &db, &mut data)?;
    } else if args.table.is_none() {
        process::process(&args, &db, &mut data)?;
        process_multimap::process_multimap(&args, &db, &mut data)?;
    } else {
        process::process(&args, &db, &mut data)?;
    }

    print::print(&args, data)?;

    Ok(())
}
