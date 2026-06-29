#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;

// Node access params
const RPC_URL: &str = "http://127.0.0.1:18443"; // Default regtest RPC port
const RPC_USER: &str = "alice";
const RPC_PASS: &str = "password";

// You can use calls not provided in RPC lib API using the generic `call` function.
// An example of using the `send` RPC call, which doesn't have exposed API.
// You can also use serde_json `Deserialize` derivation to capture the returned json result.
fn send(rpc: &Client, addr: &str) -> bitcoincore_rpc::Result<String> {
    let args = [
        json!([{addr : 20 }]), // recipient address
        json!(null),            // conf target
        json!(null),            // estimate mode
        json!(null),            // fee rate in sats/vb
        json!(null),            // Empty option object
    ];

    #[derive(Deserialize)]
    struct SendResult {
        complete: bool,
        txid: String,
    }
    let send_result = rpc.call::<SendResult>("send", &args)?;
    assert!(send_result.complete);
    Ok(send_result.txid)
}

fn main() -> bitcoincore_rpc::Result<()> {
    // Connect to Bitcoin Core RPC
    let rpc = Client::new(
        RPC_URL,
        Auth::UserPass(RPC_USER.to_owned(), RPC_PASS.to_owned()),
    )?;

    // Get blockchain info
    let blockchain_info = rpc.get_blockchain_info()?;
    println!("Blockchain Info: {:?}", blockchain_info);

    // Create/Load the wallets, named 'Miner' and 'Trader'.
    //Create wallet "Miner" and load it if it exists
    match rpc.create_wallet ("Miner", None, None, None, None) {
        Ok(_) => (),
        Err(_) => {
            rpc.load_wallet("Miner")?;
        }
    }

    //Create Wallet "Trader" and load it if ot exists
    match rpc.create_wallet ("Trader", None, None, None, None) {
        Ok(_) => (),
        Err(_) => {
            rpc.load_wallet("Trader")?;
        }
    }

    // Generate spendable balances in the Miner wallet. How many blocks needs to be mined?
    //Generate an address from the Miner wallet that will receive the  block rewards
    let miner_address = rpc.get_new_address (Some("Mining reward"), None)?;
    let address = miner_address.assume_checked();

    //101 blocks need to be mined to this address
    //1 block for reward + 100 blocks for confirmations
    rpc.generate_to_address(101, &address)?;

    //check balance
    let balance = rpc.get_balance(None)?;

    // Load Trader wallet and generate a new address
    let trader_address = rpc.get_new_address(Some("Received"), None)?;
    let trader_addr_str = trader_address.to_string();

    // Send 20 BTC from Miner to Trader
    let txid = send(&rpc, &trader_addr_str)?;

    // Check transaction in mempool
    let mempool = rpc.get_raw_mempool()?;

    // Mine 1 block to confirm the transaction
    rpc.generate_to_address(1, &address)?;

    //Get transaction details and block hash

    let tx_info = rpc.get_transaction(&txid, None)?;
    let block_height = tx_info.block_height.unwrap_or(0);
    let block_hash = rpc.get_block_hash(block_height)?;
    
    // Extract all required transaction details

    // Write the data to ../out.txt in the specified format given in readme.md
    let mut file = File::create("../out.txt")?;
    writeln!(file, "{}", txid)?;
    writeln!(file, "{}", block_height)?;
    writeln!(file, "{}", block_hash)?;
    Ok(())
}
