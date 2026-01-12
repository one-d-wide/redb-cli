use eyre::{OptionExt, Result, bail};
use redb::{
    MultimapTableDefinition, MultimapTableHandle, ReadableMultimapTable, ReadableTableMetadata,
    TableError, TypeName, Value,
};

use crate::{
    DB, Data, KVType, WARNING,
    cli::CliArgs,
    parser,
    transcode::{
        K, K_NAME, K_TREE, K_WIDTH, V, V_NAME, V_TREE, V_WIDTH, string_to_val, val_to_string,
    },
};

pub fn process_multimap(args: &CliArgs, db: &DB, data: &mut Data) -> Result<()> {
    if let Some(table) = &args.table {
        process_multimap_table(args, db, data, table)?;
    } else {
        let tables: Vec<_> = db
            .begin_read()?
            .list_multimap_tables()?
            .map(|t| t.name().to_string())
            .collect();

        for table in &tables {
            process_multimap_table(args, db, data, table)?;
        }
    };

    Ok(())
}

pub fn process_multimap_table(
    args: &CliArgs,
    db: &DB,
    data: &mut Data,
    table_name: &str,
) -> Result<()> {
    K_NAME.set(String::type_name());
    K_WIDTH.set(None);
    K_TREE.set(None);

    V_NAME.set(String::type_name());
    V_WIDTH.set(None);
    V_TREE.set(None);

    if let Some(schema) = &args.schema {
        let (k_ty, v_ty) = schema
            .split_once(" -> ")
            .ok_or_eyre("Use -> to separate key and value types")?;

        let k_tree = parser::parse_tree(k_ty)?;
        let v_tree = parser::parse_tree(v_ty)?;

        K_NAME.set(TypeName::new(k_ty));
        K_WIDTH.set(parser::parse_size(k_tree)?);

        V_NAME.set(TypeName::new(v_ty));
        V_WIDTH.set(parser::parse_size(v_tree)?);
    }

    let table_def = MultimapTableDefinition::<K, V>::new(table_name);
    for _ in 0..5 {
        match db.begin_read()?.open_multimap_table(table_def) {
            Err(TableError::TableTypeMismatch { key, value, .. }) if args.schema.is_none() => {
                K_NAME.set(key);
                V_NAME.set(value)
            }
            Err(TableError::TypeDefinitionChanged { name, width, .. }) if args.schema.is_none() => {
                if K_NAME.with_borrow(|n| n == &name) {
                    K_WIDTH.set(width);
                }
                if V_NAME.with_borrow(|n| n == &name) {
                    V_WIDTH.set(width);
                }
            }
            Err(TableError::TableDoesNotExist(_)) if args.create => {
                eprintln!("Creating table {table_name:?}");
                let w = db.begin_write()?;
                w.open_multimap_table(table_def)?;
                w.commit()?;
            }
            Err(err) => return Err(err.into()),
            Ok(_) => break,
        }
    }

    if args.list {
        let out = data.list.entry(table_name.to_string()).or_default();
        let (k, v) = (K::type_name(), V::type_name());
        out.push(k.name().to_string());
        out.push(v.name().to_string());
        return Ok(());
    }

    let k_ty = Box::leak(Box::new(String::new()));
    let v_ty = Box::leak(Box::new(String::new()));

    {
        let k = K_NAME.with_borrow(|n| n.clone());
        k_ty.push_str(k.name());
        let k_tree = parser::parse_tree(k_ty)?;

        let v = V_NAME.with_borrow(|n| n.clone());
        v_ty.push_str(v.name());

        if let Err(err) = parser::parse_tree(v_ty) {
            v_ty.clear();
            v_ty.push_str("String");
            eprintln!(
                "{WARNING} Error parsing value type {:?}, defaulting to {:?}: {err}",
                v.name(),
                v_ty,
            );
        }

        let v_tree = parser::parse_tree(v_ty)?;

        if !parser::can_order(k_tree.clone())? {
            eprintln!(
                "{WARNING} Key type {:?} of table {table_name:?} can't be ordered.",
                k.name(),
            );
            return Ok(());
        }

        // or #[cfg(false)]
        if !parser::can_order(v_tree.clone())? {
            eprintln!(
                "{WARNING} Value type {:?} of table {table_name:?} can't be ordered.",
                k.name(),
            );
            return Ok(());
        }

        K_TREE.set(Some(k_tree));
        V_TREE.set(Some(v_tree));
    }

    let KVType { k_ty, v_ty, .. } = *data.types.entry(table_name.to_string()).or_insert(KVType {
        k_ty,
        v_ty,
        is_multi: true, // or is_multi: false,
    });

    if args.delete {
        let w = db.begin_write()?;
        w.delete_multimap_table(w.open_multimap_table(table_def)?)?;
        w.commit()?;
        return Ok(());
    }

    let out = data.out.entry(table_name.to_string()).or_default();
    let r = db.begin_read()?;
    let table = r.open_multimap_table(table_def)?;

    if args.stats {
        let stat = table.stats()?;
        let stats = data.stats.entry(table_name.to_string()).or_default();
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
        return Ok(());
    }

    match (&args.key, &args.value) {
        (None, _) => {
            for r in table.iter()? {
                let (k, v) = r?;
                let k = val_to_string(k_ty, k.value());

                // or out.entry(k).or_insert(v.value());
                // or #[cfg(false)]
                let vs = out
                    .entry(k)
                    .or_insert(serde_json::Value::Array(Vec::new()))
                    .as_array_mut()
                    .unwrap();

                // or #[cfg(false)]
                for v in v {
                    vs.push(v?.value());
                }
            }
        }
        // or #[cfg(false)]
        (Some(k), Some(v)) if args.remove => {
            drop(table);
            drop(r);
            let w = db.begin_write()?;
            let mut table = w.open_multimap_table(table_def)?;
            let k = string_to_val(k_ty, k)?;
            let v = string_to_val(v_ty, v)?;
            if !table.remove(&k, &v)? {
                bail!("No such key {:?} in {table_name:?}", val_to_string(k_ty, k));
            }
            drop(table);
            w.commit()?;
            return Ok(());
        }
        (Some(k), _) if args.remove => {
            drop(table);
            drop(r);
            let w = db.begin_write()?;
            let mut table = w.open_multimap_table(table_def)?;
            let k = string_to_val(k_ty, k)?;
            if table.remove_all(&k)?.is_empty() {
                bail!("No such key {:?} in {table_name:?}", val_to_string(k_ty, k));
            }
            drop(table);
            w.commit()?;
            return Ok(());
        }
        (Some(k), None) => {
            let k = string_to_val(k_ty, k)?;
            let v = table.get(&k)?;

            if v.is_empty() {
                bail!("No such key {:?} in {table_name:?}", val_to_string(k_ty, k));
            }

            let k = val_to_string(k_ty, k);
            // or out.entry(k).or_insert(v.unwrap().value());
            // or #[cfg(false)]
            let vs = out
                .entry(k)
                .or_insert(serde_json::Value::Array(Vec::new()))
                .as_array_mut()
                .unwrap();

            // or #[cfg(false)]
            for v in v {
                vs.push(v?.value());
            }
        }
        (Some(k), Some(v)) => {
            let k = string_to_val(k_ty, k)?;
            let v = string_to_val(v_ty, v)?;

            let w = db.begin_write()?;
            let mut table = w.open_multimap_table(table_def)?;
            table.insert(k.clone(), v.clone())?;
            drop(table);
            w.commit()?;
        }
    }

    Ok(())
}
