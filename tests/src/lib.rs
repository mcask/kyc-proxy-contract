use std::collections::BTreeMap;
use std::path::PathBuf;

use casper_engine_test_support::{
    DeployItemBuilder, ExecuteRequestBuilder, InMemoryWasmTestBuilder, WasmTestBuilder, ARG_AMOUNT,
    DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT, DEFAULT_RUN_GENESIS_REQUEST,
};

use casper_execution_engine::storage::global_state::in_memory::InMemoryGlobalState;
use casper_types::system::mint;
use casper_types::{account::AccountHash, runtime_args, PublicKey, RuntimeArgs, SecretKey, U512};
use casper_types::{ContractHash, ContractPackageHash, Key};

pub struct ProxyContract {
    pub builder: WasmTestBuilder<InMemoryGlobalState>,
    pub contract_hash: [u8; 32],
    pub package_hash: ContractPackageHash,
    pub admin_account: (PublicKey, AccountHash),
    pub participant_two: (PublicKey, AccountHash),
    pub participant_three: (PublicKey, AccountHash),
}

impl ProxyContract {
    pub fn deploy() -> Self {
        // We create 3 users. One to oversee and deploy the contract, one to send the payment
        // and one to receive it.
        let admin_public_key: PublicKey =
            (&SecretKey::ed25519_from_bytes([1u8; 32]).unwrap()).into();
        let participant_two_public_key: PublicKey =
            (&SecretKey::ed25519_from_bytes([2u8; 32]).unwrap()).into();
        let participant_three_public_key: PublicKey =
            (&SecretKey::ed25519_from_bytes([3u8; 32]).unwrap()).into();
        // Get addresses for participating users.
        let admin_account_addr = AccountHash::from(&admin_public_key);
        let participant_two_account_addr = AccountHash::from(&participant_two_public_key);
        let participant_three_account_addr = AccountHash::from(&participant_three_public_key);

        let code = PathBuf::from("kyc-proxy.wasm");
        let args = runtime_args! {"initial_providers"=> Option::<Vec<ContractPackageHash>>::None};
        let deploy = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_session_code(code, args)
            .with_address(admin_account_addr)
            .with_authorization_keys(&[admin_account_addr])
            .build();
        let execute_request = ExecuteRequestBuilder::from_deploy_item(deploy).build();

        let mut builder = InMemoryWasmTestBuilder::default();
        builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();

        let fund_my_account_request = {
            let deploy_item = DeployItemBuilder::new()
                .with_address(*DEFAULT_ACCOUNT_ADDR)
                .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
                .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
                .with_transfer_args(runtime_args! {
                    mint::ARG_AMOUNT => U512::from(30_000_000_000_000_u64),
                    mint::ARG_TARGET => admin_public_key.clone(),
                    mint::ARG_ID => <Option::<u64>>::None
                })
                .with_deploy_hash([1; 32])
                .build();

            ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
        };
        builder
            .exec(fund_my_account_request)
            .expect_success()
            .commit();

        let fund_my_account_request = {
            let deploy_item = DeployItemBuilder::new()
                .with_address(*DEFAULT_ACCOUNT_ADDR)
                .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
                .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
                .with_transfer_args(runtime_args! {
                    mint::ARG_AMOUNT => U512::from(30_000_000_000_000_u64),
                    mint::ARG_TARGET => participant_two_public_key.clone(),
                    mint::ARG_ID => <Option::<u64>>::None
                })
                .with_deploy_hash([1; 32])
                .build();

            ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
        };
        builder
            .exec(fund_my_account_request)
            .expect_success()
            .commit();

        let fund_my_account_request = {
            let deploy_item = DeployItemBuilder::new()
                .with_address(*DEFAULT_ACCOUNT_ADDR)
                .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
                .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
                .with_transfer_args(runtime_args! {
                    mint::ARG_AMOUNT => U512::from(30_000_000_000_000_u64),
                    mint::ARG_TARGET => participant_three_public_key.clone(),
                    mint::ARG_ID => <Option::<u64>>::None
                })
                .with_deploy_hash([1; 32])
                .build();

            ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
        };
        builder
            .exec(fund_my_account_request)
            .expect_success()
            .commit();

        builder.exec(execute_request).commit().expect_success();

        // make assertions
        let contract_hash = builder
            .query(
                None,
                Key::Account(admin_account_addr),
                &["kyc-proxy_contract_hash".to_string()],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("should be string.");
        let package_hash = builder
            .query(
                None,
                Key::Account(admin_account_addr),
                &[
                    "kyc-proxy_contract".to_string(),
                    "kyc-proxy_contract_package".to_string(),
                ],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("should be string.");

        // let clx_init_balance = U512::from(500_000_000_000_000_000u64);
        // let mut context = DeployItemBuilder::default()
        //     .with_public_key(admin_public_key.clone(), clx_init_balance)
        //     .with_public_key(participant_two_public_key.clone(), clx_init_balance)
        //     .with_public_key(participant_three_public_key.clone(), clx_init_balance)
        //     .build();

        Self {
            builder,
            contract_hash,
            package_hash,
            admin_account: (admin_public_key, admin_account_addr),
            participant_two: (participant_two_public_key, participant_two_account_addr),
            participant_three: (participant_three_public_key, participant_three_account_addr),
        }
    }

    pub fn deploy_kyc(
        &mut self,
        deployer: AccountHash,
        kyc_name: &str,
    ) -> (ContractPackageHash, ContractHash) {
        let kyc_code = PathBuf::from("kyc-contract.wasm");
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "kyc".to_string());

        let kyc_args = runtime_args! {
            "name" => kyc_name,
            "contract_name" => "kyc",
            "symbol" => "symbol",
            "meta" => meta,
            "admin" => Key::Account(deployer)
        };
        let kyc_session = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_session_code(kyc_code, kyc_args)
            .with_address(deployer)
            .with_authorization_keys(&[deployer])
            .build();
        let execute_request = ExecuteRequestBuilder::from_deploy_item(kyc_session).build();
        self.builder.exec(execute_request).commit();
        (
            self.builder
                .query(
                    None,
                    Key::Account(deployer),
                    &["kyc_package_hash_wrapped".to_string()],
                )
                .expect("should be stored value.")
                .as_cl_value()
                .expect("should be cl value.")
                .clone()
                .into_t()
                .expect("should be string."),
            self.builder
                .query(
                    None,
                    Key::Account(deployer),
                    &["kyc_contract_hash_wrapped".to_string()],
                )
                .expect("should be stored value.")
                .as_cl_value()
                .expect("should be cl value.")
                .clone()
                .into_t()
                .expect("should be string."),
        )
    }

    pub fn add_kyc(&mut self, deployer: AccountHash, kyc_hash: [u8; 32], recipient: AccountHash) {
        let mut token_meta = BTreeMap::new();
        token_meta.insert("status".to_string(), "active".to_string());
        let args = runtime_args! {
            "recipient" => Key::Account(recipient),
            "token_id" => Some(recipient.to_string()),
            "token_meta" => token_meta
        };
        let deploy = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_stored_versioned_contract_by_hash(kyc_hash, None, "mint", args)
            .with_address(deployer)
            .with_authorization_keys(&[deployer])
            .build();
        let execute_request = ExecuteRequestBuilder::from_deploy_item(deploy).build();
        self.builder.exec(execute_request).commit();
    }

    /// Getter function for the balance of an account.
    fn get_balance(&self, account_key: &AccountHash) -> U512 {
        let account = self
            .builder
            .get_account(*account_key)
            .expect("should get genesis account");
        self.builder.get_purse_balance(account.main_purse())
    }

    /// Shorthand to get the balances of all 3 accounts in order.
    pub fn get_all_accounts_balance(&self) -> (U512, U512, U512) {
        (
            self.get_balance(&self.admin_account.1),
            self.get_balance(&self.participant_two.1),
            self.get_balance(&self.participant_three.1),
        )
    }

    /// Function that handles the creation and running of sessions.
    fn call(&mut self, caller: AccountHash, method: &str, args: RuntimeArgs) {
        let deploy = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_stored_versioned_contract_by_hash(self.contract_hash, None, method, args)
            .with_address(caller)
            .with_authorization_keys(&[caller])
            .build();
        let execute_request = ExecuteRequestBuilder::from_deploy_item(deploy).build();
        self.builder.exec(execute_request).commit();
    }

    pub fn add_kyc_provider(&mut self, provider_package_hash_key: ContractPackageHash) {
        self.call(
            self.admin_account.1,
            "add_kyc_provider",
            runtime_args! {"provider"=>Key::Hash(provider_package_hash_key.value())},
        );
    }

    pub fn ban_provider(&mut self, provider_package_hash_key: ContractPackageHash) {
        self.call(
            self.admin_account.1,
            "ban_provider",
            runtime_args! {"provider"=>Key::Hash(provider_package_hash_key.value())},
        );
    }

    pub fn unban_provider(&mut self, provider_package_hash_key: ContractPackageHash) {
        self.call(
            self.admin_account.1,
            "unban_provider",
            runtime_args! {"provider"=>Key::Hash(provider_package_hash_key.value())},
        );
    }

    pub fn is_kyc_proved(&mut self, result: bool) -> &mut WasmTestBuilder<InMemoryGlobalState> {
        let code = PathBuf::from("test_contract.wasm");
        let deploy = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_session_code(
                code,
                runtime_args! {
                    "kyc_proxy_package_hash"=>self.package_hash,
                    "result" => result
                },
            )
            .with_address(self.admin_account.1)
            .with_authorization_keys(&[self.admin_account.1])
            .build();
        let execute_request = ExecuteRequestBuilder::from_deploy_item(deploy).build();
        self.builder.exec(execute_request).commit()
    }
}

#[test]
fn test_deploy() {
    ProxyContract::deploy();
}

#[test]
fn test_no_provider() {
    let mut proxy = ProxyContract::deploy();
    proxy.is_kyc_proved(false);
}

#[test]
#[should_panic = "User(999)"]
fn test_no_provider_failing() {
    let mut proxy = ProxyContract::deploy();
    proxy.is_kyc_proved(true).expect_success();
}

#[test]
fn test_single_provider_proxy_negative() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, _first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.is_kyc_proved(false);
}

#[test]
fn test_single_provider_proxy_positive() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.add_kyc(
        proxy.participant_two.1,
        first_provider_hash.value(),
        proxy.admin_account.1,
    );
    proxy.is_kyc_proved(true);
}

#[test]
fn test_multiple_provider_proxy_negative() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, _first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    let (second_provider_package_hash, _second_provider_hash) =
        proxy.deploy_kyc(proxy.participant_three.1, "second");
    proxy.add_kyc_provider(second_provider_package_hash);
    proxy.is_kyc_proved(false);
}

#[test]
fn test_multiple_provider_proxy_first_positive() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    let (second_provider_package_hash, _second_provider_hash) =
        proxy.deploy_kyc(proxy.participant_three.1, "second");

    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.add_kyc_provider(second_provider_package_hash);

    proxy.add_kyc(
        proxy.participant_two.1,
        first_provider_hash.value(),
        proxy.admin_account.1,
    );
    proxy.is_kyc_proved(true);
}

#[test]
fn test_multiple_provider_proxy_second_positive() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, _first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);

    let (second_provider_package_hash, second_provider_hash) =
        proxy.deploy_kyc(proxy.participant_three.1, "second");
    proxy.add_kyc_provider(second_provider_package_hash);

    proxy.add_kyc(
        proxy.participant_three.1,
        second_provider_hash.value(),
        proxy.admin_account.1,
    );
    proxy.is_kyc_proved(true);
}

#[test]
fn test_banned_provider() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.add_kyc(
        proxy.participant_two.1,
        first_provider_hash.value(),
        proxy.admin_account.1,
    );
    proxy.ban_provider(first_provider_package_hash);
    proxy.is_kyc_proved(false);
}

#[test]
fn test_unbanned_provider() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, first_provider_hash) =
        proxy.deploy_kyc(proxy.participant_two.1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.add_kyc(
        proxy.participant_two.1,
        first_provider_hash.value(),
        proxy.admin_account.1,
    );
    proxy.ban_provider(first_provider_package_hash);
    proxy.is_kyc_proved(false);
    proxy.unban_provider(first_provider_package_hash);
    proxy.is_kyc_proved(true);
}
