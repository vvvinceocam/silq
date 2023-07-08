#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;

/// Proof of concept function
#[php_function]
pub fn hello_world_from_rust(name: String) -> String {
    format!("Hello, {}!", name)
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
}
