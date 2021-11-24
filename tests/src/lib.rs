use std::collections::BTreeMap;

use casper_engine_test_support::{Code, Hash, SessionBuilder, TestContext, TestContextBuilder};
use casper_types::{account::AccountHash, runtime_args, PublicKey, RuntimeArgs, SecretKey, U512};
use casper_types::{ContractHash, ContractPackageHash, Key};

pub struct ProxyContract {
    pub context: TestContext,
    pub contract_hash: Hash,
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

        // create context with cash for all users
        let clx_init_balance = U512::from(500_000_000_000_000_000u64);
        let mut context = TestContextBuilder::new()
            .with_public_key(admin_public_key.clone(), clx_init_balance)
            .with_public_key(participant_two_public_key.clone(), clx_init_balance)
            .with_public_key(participant_three_public_key.clone(), clx_init_balance)
            .build();

        // load contract into context
        let code = Code::from("kyc-proxy.wasm");
        let args = runtime_args! {"initial_providers"=> Option::<Vec<ContractPackageHash>>::None};
        let session = SessionBuilder::new(code, args)
            .with_address(admin_account_addr)
            .with_authorization_keys(&[admin_account_addr])
            .build();
        context.run(session);

        let contract_hash = context
            .query(admin_account_addr, &["kyc-proxy_contract_hash".to_string()])
            .unwrap_or_else(|_| panic!("kyc-proxy_contract_hash contract not found"))
            .into_t()
            .unwrap_or_else(|_| panic!("kyc-proxy_contract_hash has wrong type"));

        let package_hash: ContractPackageHash = context
            .query(
                admin_account_addr,
                &[
                    "kyc-proxy_contract".to_string(),
                    "kyc-proxy_contract_package".to_string(),
                ],
            )
            .unwrap()
            .into_t()
            .unwrap();

        Self {
            context,
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
        let kyc_code = Code::from("kyc-contract.wasm");
        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "kyc".to_string());

        let kyc_args = runtime_args! {
            "name" => kyc_name,
            "contract_name" => "kyc",
            "symbol" => "symbol",
            "meta" => meta,
            "admin" => Key::Account(deployer)
        };
        let kyc_session = SessionBuilder::new(kyc_code, kyc_args)
            .with_address(deployer)
            .with_authorization_keys(&[deployer])
            .build();

        self.context.run(kyc_session);
        (
            self.context
                .query(deployer, &["kyc_package_hash_wrapped".to_string()])
                .unwrap()
                .into_t()
                .unwrap(),
            self.context
                .query(deployer, &["kyc_contract_hash_wrapped".to_string()])
                .unwrap()
                .into_t()
                .unwrap(),
        )
    }

    pub fn add_kyc(&mut self, deployer: AccountHash, kyc_hash: Hash, recipient: AccountHash) {
        let code = Code::Hash(kyc_hash, "mint".to_string());
        let mut token_meta = BTreeMap::new();
        token_meta.insert("status".to_string(), "active".to_string());
        let args = runtime_args! {
            "recipient" => Key::Account(recipient),
            "token_id" => Some(recipient.to_string()),
            "token_meta" => token_meta
        };
        let session = SessionBuilder::new(code, args)
            .with_address(deployer)
            .with_authorization_keys(&[deployer])
            .build();
        self.context.run(session);
    }

    /// Getter function for the balance of an account.
    fn get_balance(&self, account_key: &AccountHash) -> U512 {
        let main_purse_address = self.context.main_purse_address(*account_key).unwrap();
        self.context.get_balance(main_purse_address.addr())
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
        let code = Code::Hash(self.contract_hash, method.to_string());
        let session = SessionBuilder::new(code, args)
            .with_address(caller)
            .with_authorization_keys(&[caller])
            .build();
        self.context.run(session);
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

    pub fn is_kyc_proved(&mut self, result: bool) {
        let code = Code::from("test_contract.wasm");
        let session = SessionBuilder::new(
            code,
            runtime_args! {
                "kyc_proxy_package_hash"=>self.package_hash,
                "result" => result
            },
        )
        .with_address(self.admin_account.1)
        .with_authorization_keys(&[self.admin_account.1])
        .build();
        self.context.run(session);
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
    proxy.is_kyc_proved(true);
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
