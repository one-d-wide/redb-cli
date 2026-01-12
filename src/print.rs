use std::io::Write;

use crate::{Data, KVType, cli::CliArgs, transcode::val_to_string};

pub fn print(args: &CliArgs, data: Data) -> eyre::Result<()> {
    if args.delete || args.remove {
        return Ok(());
    }

    let Data {
        stats,
        list,
        out,
        types,
    } = data;

    if args.json {
        let mut stdout = std::io::stdout().lock();

        match (&args.table, &args.key, &args.value) {
            _ if args.list => serde_json::to_writer_pretty(&mut stdout, &list)?,
            _ if args.stats => serde_json::to_writer_pretty(&mut stdout, &stats)?,
            (None, _, _) => serde_json::to_writer_pretty(&mut stdout, &out)?,
            (Some(t), None, _) => serde_json::to_writer_pretty(&mut stdout, &out[t])?,
            (Some(t), Some(k), None) => serde_json::to_writer_pretty(&mut stdout, &out[t][k])?,
            (Some(_t), Some(_k), Some(_v)) => return Ok(()),
        }
        writeln!(stdout)?;
        return Ok(());
    }

    if args.list {
        for (table, types) in &list {
            println!("{table}: {} -> {}", types[0], types[1]);
        }
        return Ok(());
    }

    if args.stats {
        for (table, out) in &stats {
            println!();
            println!("{table}:");
            for (k, v) in out {
                println!("{k}: {v}");
            }
        }
        return Ok(());
    }

    for (table, out) in out {
        if args.table.is_none() {
            println!();
            println!("{table}:");
        }
        let KVType { v_ty, is_multi, .. } = types.get(&table).unwrap();
        for (k, v) in out {
            if *is_multi {
                for v in v.as_array().unwrap() {
                    let v = val_to_string(v_ty, v.clone());
                    if args.key.is_none() {
                        print!("{k}: ");
                    }
                    println!("{v}");
                }
            } else {
                let v = val_to_string(v_ty, v);
                if args.key.is_none() {
                    print!("{k}: ");
                }
                println!("{v}");
            }
        }
    }

    Ok(())
}
