use duckdb::{Connection, Result};
use hello::load_extension;

fn main() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    load_extension(&conn)?;
    let val = conn.query_row("select * from hello('Alice', count=1)", [], |row| {
        <(String,)>::try_from(row)
    })?;
    println!("got: {:?}", val);
    Ok(())
}
