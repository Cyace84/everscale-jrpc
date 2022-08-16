use ed25519_dalek::Signer;
use everscale_jrpc_client::{LoadBalancedRpcOptions, SendOptions, TransportErrorAction};
use nekoton::core::models::Expiration;
use nekoton::core::ton_wallet::multisig::prepare_transfer;
use nekoton::core::ton_wallet::{Gift, MultisigType, TransferAction, WalletType};
use nekoton::crypto::MnemonicType;
use nekoton_utils::SimpleClock;
use std::str::FromStr;
use std::time::Duration;
use ton_block::MsgAddressInt;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter(Some("everscale_jrpc_client"), log::LevelFilter::Debug)
        .init();
    let seed = std::env::args().nth(1).unwrap();
    let to = std::env::args().nth(2).unwrap();

    let to = MsgAddressInt::from_str(&to).expect("invalid address");

    let client = everscale_jrpc_client::LoadBalancedRpc::new(
        ["https://extension-api.broxus.com/rpc".parse().unwrap()],
        LoadBalancedRpcOptions {
            prove_interval: Duration::from_secs(10),
        },
    )
    .await
    .unwrap();

    let signer =
        nekoton::crypto::derive_from_phrase(&seed, MnemonicType::Labs(0)).expect("invalid seed");
    let from = nekoton::core::ton_wallet::compute_address(
        &signer.public,
        WalletType::Multisig(MultisigType::SafeMultisigWallet),
        0,
    );

    let tx = prepare_transfer(
        &SimpleClock,
        &signer.public,
        false,
        from,
        Gift {
            flags: 3,
            bounce: false,
            destination: to,
            amount: 1_000_000_000,
            /// can be built with `nekoton_abi::MessageBuilder`
            body: None,
            state_init: None,
        },
        Expiration::Timeout(60),
    )
    .unwrap();

    let message = match tx {
        TransferAction::DeployFirst => {
            panic!("DeployFirst not supported")
        }
        TransferAction::Sign(m) => m,
    };

    let signature = signer.sign(message.hash()).to_bytes();
    let signed_message = message.sign(&signature).expect("invalid signature");

    let message = signed_message.message;
    let send_options = SendOptions {
        error_action: TransportErrorAction::Return,
        ttl: Duration::from_secs(60),
        poll_interval: Duration::from_secs(1),
    };

    let status = client
        .send_with_confirmation(message, send_options)
        .await
        .expect("failed to send message");
    println!("status: {:?}", status);
}