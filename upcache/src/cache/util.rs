use crate::prelude::*;

use rusqlite::{Connection, params, OpenFlags, types::FromSql};

pub fn load_rows(db: &mut Connection, tbl: &str, col: &str, order: &str,
                 mut f: impl FnMut(&rusqlite::Row) -> Result<()>) -> Result<()>
{
    let mut stmt = db.prepare(&format!("SELECT {} from {} ORDER BY {}", col, tbl, order))?;
    let mut rows = stmt.query(params![])?;
    while let Some(row) = rows.next().expect("") {
        f(row)?
    }
    Ok(())
}

pub fn load_vec<T: FromSql>(db: &mut Connection, tbl: &str, col: &str, order: &str) -> Result<Vec<T>> {
    let mut res = vec![];
    load_rows(db, tbl, col, order, |r| {
        res.push(r.get(0)?);
        Ok(())
    })?;
    Ok(res)
}

pub fn load_map<K: FromSql + Eq + Hash + Ord, V: FromSql>(db: &mut Connection, tbl: &str, key_col: &str, val_col: &str, order: &str) -> Result<Map<K, V>> {
    let a = load_vec(db, tbl, key_col, order)?;
    let b = load_vec(db, tbl, val_col, order)?;
    return Ok((a.into_iter().zip(b.into_iter())).collect());
}

pub fn load_multimap<K: FromSql + Eq + Hash + Ord, V: FromSql>(db: &mut Connection, tbl: &str, key_col: &str, val_col: &str, order: &str) -> Result<Map<K, Vec<V>>> {
    let a = load_vec(db, tbl, key_col, order)?;
    let b = load_vec(db, tbl, val_col, order)?;

    let mut res: Map<K, Vec<V>> = Map::default();
    for (a, b) in a.into_iter().zip(b.into_iter()) {
        res.entry(a).or_default().push(b);
    }
    return Ok(res);
}

pub fn invert<K: Hash + Eq + Clone + Ord, V: Hash + Eq + Clone + Ord>(v: &Map<K, V>) -> Map<V, K> {
    v.iter().map(|(a, b)| (b.clone(), a.clone())).collect()
}
