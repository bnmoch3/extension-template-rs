use duckdb::core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId};
use duckdb::vtab::{BindInfo, Free, FunctionInfo, InitInfo, VTab, VTabWithLocalData};
use duckdb::Result;

#[cfg(feature = "loadable-extension")]
use duckdb_loadable_macros::duckdb_entrypoint_c_api;

#[cfg(feature = "loadable-extension")]
use duckdb::{ffi, Connection};

use std::ffi::{c_char, CString};

#[derive(Debug)]
#[repr(C)]
struct HelloBindData {
    name: *mut c_char,
    count: u32,
}

impl Free for HelloBindData {
    fn free(&mut self) {
        unsafe {
            if self.name.is_null() {
                return;
            }
            drop(CString::from_raw(self.name));
        }
    }
}

#[repr(C)]
struct HelloGlobalData {
    done: bool,
}

impl Free for HelloGlobalData {}

#[repr(C)]
#[derive(Debug)]
struct HelloLocalData {
    remaining: u32,
}

impl Free for HelloLocalData {}

struct HelloVTab;

impl VTab for HelloVTab {
    type BindData = HelloBindData;
    type GlobalData = HelloGlobalData;

    fn name() -> &'static str {
        "hello"
    }

    unsafe fn bind(
        info: &BindInfo,
        bind_data: *mut HelloBindData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // params
        let bind_data = unsafe { &mut *bind_data };
        let name_param = info.get_parameter(0).to_string();
        let count_param = info
            .get_named_parameter("count")
            .map_or(1, |v| v.to_int64());
        bind_data.name = CString::new(name_param).unwrap().into_raw();
        bind_data.count = count_param.try_into()?;

        // schema
        info.add_result_column("greetings", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        Ok(())
    }

    unsafe fn init(
        _: &InitInfo,
        data: *mut HelloGlobalData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = unsafe { &mut *data };
        data.done = false;
        Ok(())
    }

    unsafe fn func(
        func: &FunctionInfo,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_info = &mut *func.get_bind_data::<HelloBindData>();
        let global_state = &mut *func.get_init_data::<HelloGlobalData>();
        let local_state = &mut *func.get_local_init_data::<HelloLocalData>();

        if global_state.done {
            output.set_len(0);
        } else {
            let names_vec = output.flat_vector(0);
            let name = CString::from_raw(bind_info.name);
            let result = CString::new(format!(
                "Hello {} {}",
                name.to_str()?,
                local_state.remaining
            ))?;
            bind_info.name = CString::into_raw(name);
            names_vec.insert(0, result);
            output.set_len(1);
            local_state.remaining -= 1;
            if local_state.remaining == 0 {
                global_state.done = true;
            }
        }
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)])
    }

    fn named_parameters() -> Option<Vec<(String, LogicalTypeHandle)>> {
        Some(vec![(
            "count".to_string(),
            LogicalTypeHandle::from(LogicalTypeId::Bigint),
        )])
    }
}

impl VTabWithLocalData for HelloVTab {
    type LocalData = HelloLocalData;
    unsafe fn init_local(
        init_info: &InitInfo,
        local_data: *mut HelloLocalData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_data: &HelloBindData = unsafe { &*init_info.get_bind_data() };
        let local_data = unsafe { &mut *local_data };
        local_data.remaining = bind_data.count;
        Ok(())
    }
}

pub fn load_extension(conn: &duckdb::Connection) -> duckdb::Result<()> {
    conn.register_table_function_with_local_init::<HelloVTab>()?;
    Ok(())
}

#[cfg(feature = "loadable-extension")]
#[duckdb_entrypoint_c_api(ext_name = "hello", min_duckdb_version = "v0.0.1")]
pub unsafe fn extension_entrypoint(conn: Connection) -> Result<(), Box<dyn std::error::Error>> {
    load_extension(&conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::load_extension;
    use duckdb::{params, Connection};

    #[test]
    fn test_basic_usage() -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        let version: String = conn
            .query_row("select version()", params![], |r| r.get(0))
            .unwrap();
        println!("Duckdb Version: {}", version);

        load_extension(&conn)?;

        let name = "Alice";
        let count: i64 = 10;
        let got = conn.query_row(
            "select count(*) from hello(?, count=?)",
            params![name, count],
            |row| <(i64,)>::try_from(row),
        )?;
        assert_eq!(count, got.0);
        Ok(())
    }
}
