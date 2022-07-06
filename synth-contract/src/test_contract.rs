#![no_main]
#![no_std]

use casper_contract::contract_api::runtime::{
    call_versioned_contract, get_caller, get_named_arg, revert,
};
use casper_types::{runtime_args, ApiError, Key, RuntimeArgs, U256};

#[no_mangle]
pub extern "C" fn call() {
    if get_named_arg::<bool>("result")
        != call_versioned_contract::<bool>(
            get_named_arg("synth_package_hash"),
            None,
            "is_enabled",
            runtime_args! {
                "account" => Key::Account(get_caller()),
                "index" => Option::<U256>::None
            },
        )
    {
        revert(ApiError::User(999))
    }
}
