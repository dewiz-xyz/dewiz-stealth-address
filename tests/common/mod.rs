pub mod smartcontract;

use alloy::network::{AnyNetwork, EthereumWallet, NetworkWallet};
use alloy::primitives::Address;
use alloy::providers::fillers::{
    BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Identity, ProviderBuilder, RootProvider, WalletProvider};
use alloy::signers::local::PrivateKeySigner;
use dewiz_stealth_address::stealth::StealthMetaAddress;
use dotenvy::dotenv;
use std::sync::Once;

use self::smartcontract::abi::ERC20::ERC20Instance;

static TRACING_INIT: Once = Once::new();

pub type AppFiller = JoinFill<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    WalletFiller<EthereumWallet>,
>;

#[allow(dead_code)]
pub struct TestApp {
    pub client_receiver: FillProvider<AppFiller, RootProvider>,
    pub client_sender: FillProvider<AppFiller, RootProvider>,
    pub erc20_destination_contract_address: Address,
    pub erc20_attached_to_sender_wallet: ERC20Instance<FillProvider<AppFiller, RootProvider>>,
    pub stealth_key_receiver: StealthMetaAddress,
    pub stealth_key_sender: StealthMetaAddress,
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl TestApp {
    pub fn new() -> Self {
        dotenv().ok();

        TRACING_INIT.call_once(|| {
            let _ = tracing_subscriber::fmt::try_init();
        });

        // Environment configuration
        let rpc_url = std::env::var("RPC_URL")
            .expect("RPC_URL must be defined in .env or as an environment variable");
        let private_key_sender = std::env::var("PRIVATE_KEY_SENDER")
            .expect("PRIVATE_KEY_SENDER must be defined in .env or as an environment variable");
        let private_key_receiver = std::env::var("PRIVATE_KEY_RECEIVER")
            .expect("PRIVATE_KEY_RECEIVER must be defined in .env or as an environment variable");
        let usdc_contract_address: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .to_owned()
            .parse()
            .expect("Invalid contract address"); // USDC on Ethereum Mainnet

        // Wallet configuration
        let signer_sender: PrivateKeySigner = private_key_sender
            .parse()
            .expect("The private key must be valid");
        let signer_receiver: PrivateKeySigner = private_key_receiver
            .parse()
            .expect("The private key must be valid");

        // Derive stealth meta-addresses from the EOA private keys
        let stealth_key_sender = StealthMetaAddress::from_secp256k1_nonzeroscalar(
            signer_sender.credential().as_nonzero_scalar(),
        );
        let stealth_key_receiver = StealthMetaAddress::from_secp256k1_nonzeroscalar(
            signer_receiver.credential().as_nonzero_scalar(),
        );

        let wallet_signer_sender = EthereumWallet::from(signer_sender);
        let wallet_signer_receiver = EthereumWallet::from(signer_receiver);

        let provider_client_receiver = ProviderBuilder::new()
            .wallet(wallet_signer_receiver.clone())
            .connect_http(rpc_url.parse().expect("RPC_URL must be a valid URL"));

        let provider_client_sender = ProviderBuilder::new()
            .wallet(wallet_signer_sender.clone())
            .connect_http(rpc_url.parse().expect("RPC_URL must be a valid URL"));

        let erc20_instance_attached_to_sender_wallet =
            ERC20Instance::new(usdc_contract_address, provider_client_sender.clone());

        Self {
            client_receiver: provider_client_receiver,
            client_sender: provider_client_sender,
            erc20_destination_contract_address: usdc_contract_address,
            erc20_attached_to_sender_wallet: erc20_instance_attached_to_sender_wallet,
            stealth_key_receiver,
            stealth_key_sender,
        }
    }

    pub async fn get_receiver_address(&self) -> Address {
        <EthereumWallet as NetworkWallet<AnyNetwork>>::default_signer_address(
            self.client_receiver.wallet(),
        )
    }

    pub async fn get_sender_address(&self) -> Address {
        <EthereumWallet as NetworkWallet<AnyNetwork>>::default_signer_address(
            self.client_sender.wallet(),
        )
    }
}
