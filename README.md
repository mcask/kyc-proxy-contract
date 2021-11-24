# KYC Proxy Contract
This contract is compatible with kyc contracts that have the following entrypoint:
```
EntryPoint::new(
    "is_kyc_proved",
    vec![
        Parameter::new("account", Key::cl_type()),
        Parameter::new("index", CLType::Option(Box::new(U256::cl_type()))),
    ],
    CLType::Bool,
    EntryPointAccess::Public,
    EntryPointType::Contract,
)
```

The proxy contract accepts a list of `contract_package_hash` on install deploy or singular package hashes on later deploys when calling the `"add_provider"` entrypoint.
These providers can be banned or unbanned. Banned providers will not be asked for their opinion.

### Versions
This example is on casper-types and casper-contract version 1.4.1
rustc 1.58.0-nightly (00d5e42e7 2021-10-24)