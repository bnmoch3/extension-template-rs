extern crate duckdb;
extern crate duckdb_loadable_macros;
extern crate libduckdb_sys;

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, Free, FunctionInfo, InitInfo, VTab, VTabWithLocalData},
    Connection, Result,
};
use duckdb_loadable_macros::duckdb_entrypoint_c_api;
use libduckdb_sys as ffi;
use std::{
    error::Error,
    ffi::{c_char, CString},
};

#[repr(C)]
struct HelloBindData {
    name: *mut c_char,
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

struct Hello;

impl VTab for Hello {
    type GlobalData = HelloGlobalData;
    type BindData = HelloBindData;

    unsafe fn bind(
        bind: &BindInfo,
        data: *mut HelloBindData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        bind.add_result_column("value", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        let param = bind.get_parameter(0).to_string();
        unsafe {
            (*data).name = CString::new(param).unwrap().into_raw();
        }
        Ok(())
    }

    unsafe fn init(
        _: &InitInfo,
        data: *mut HelloGlobalData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            (*data).done = false;
        }
        Ok(())
    }

    unsafe fn func(
        func: &FunctionInfo,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_info = func.get_bind_data::<HelloBindData>();
        let global_state = &mut *func.get_init_data::<HelloGlobalData>();
        let local_state = &mut *func.get_local_init_data::<HelloLocalData>();

        if global_state.done {
            output.set_len(0);
        } else {
            let vector = output.flat_vector(0);
            let name = CString::from_raw((*bind_info).name);
            let num = local_state.remaining;
            let result = CString::new(format!("Hello {} {}", name.to_str()?, num))?;
            (*bind_info).name = CString::into_raw(name); // Can't consume the CString
            vector.insert(0, result);
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
}

#[repr(C)]
struct HelloLocalData {
    remaining: i32,
}
impl Free for HelloLocalData {}
impl VTabWithLocalData for Hello {
    type LocalData = HelloLocalData;
    unsafe fn init_local(
        _init: &InitInfo,
        data: *mut HelloLocalData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            (*data).remaining = 5;
        }
        Ok(())
    }
}

const EXTENSION_NAME: &str = env!("CARGO_PKG_NAME");

#[duckdb_entrypoint_c_api(ext_name = "rusty_quack", min_duckdb_version = "v0.0.1")]
pub unsafe fn extension_entrypoint(conn: Connection) -> Result<(), Box<dyn Error>> {
    conn.register_table_function_with_local_init::<Hello>(EXTENSION_NAME)
        .expect("Failed to register hello table function");
    Ok(())
}
