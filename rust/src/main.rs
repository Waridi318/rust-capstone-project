#![allow(unused)]
use bitcoin::hex::DisplayHex;
use bitcoincore_rpc::bitcoin::Amount;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use serde::Deserialize;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use bitcoincore_rpc::bitcoin::Txid;
use std::str::FromStr;
use bitcoincore_rpc::bitcoin::{Address, Network};

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
    let balance = rpc.get_balance(None, None)?;

    // Load Trader wallet and generate a new address
    let trader_address = rpc.get_new_address(Some("Received"), None)?;
    let trader_addr_checked = trader_address.assume_checked();
    let trader_addr_str = trader_addr_checked.to_string();

    // Send 20 BTC from Miner to Trader
    let txid = send(&rpc, &trader_addr_str)?;

    // Check transaction in mempool
    let mempool = rpc.get_raw_mempool()?;

    // Mine 1 block to confirm the transaction
    rpc.generate_to_address(1, &address)?;

    //Get transaction details and block hash
    let txid_parsed = Txid::from_str(&txid).expect("Invalid txid format");
    let tx_info = rpc.get_transaction(&txid_parsed, None)?;
    let block_height = tx_info.info.blockheight.unwrap_or(0);
    let block_hash = rpc.get_block_hash(block_height.into())?;

    //Get the raw transaction to analyze inouts and outputs
    let raw_tx = rpc.get_raw_transaction(&txid_parsed, None)?;

    //Get input address and amount from the coinbase transaction
    //this is the vector of inputs
    let first_input = &raw_tx.input[0];
    //previous transaction (the coinbase tx)
    let prev_txid = first_input.previous_output.txid;
    let prev_vout = first_input.previous_output.vout as usize;
    
    let prev_tx = rpc.get_raw_transaction(&prev_txid, None)?;
    
    //extract the input address and amount from orevious transaction output
    //this is the vector of outputs
    let prev_output = &prev_tx.output[prev_vout];
    //get the address from the script_pubkey
    let input_address = Address::from_script(&prev_output.script_pubkey, Network::Regtest)
        .expect("No address found in input")
        .to_string();
    
    //get the amount
    let input_amount = prev_output.value.to_btc();

    //extract trader output and change output
    let mut trader_output_amount = 0.0;
    let mut miner_change_address = String::new();
    let mut miner_change_amount = 0.0;

    //loop through all the outputs
    for output in &raw_tx.output {
        //get the address from the script_pubkey
        if let Ok(addr) = Address::from_script(&output.script_pubkey, Network::Regtest){
            let addr_str = addr.to_string();

            //check if this output goes to trader
            if addr_str == trader_addr_str{
                trader_output_amount = output.value.to_btc();
            }
            else {
                miner_change_address = addr_str;
                miner_change_amount = output.value.to_btc();
            }
        }
    }
    
    //calculate transaction fee
    let total_output = trader_output_amount + miner_change_amount;
    let fee = input_amount - total_output;

    // Write the data to ../out.txt in the specified format given in readme.md
    let mut file = File::create("../out.txt")?;
    writeln!(file, "{}", txid)?;
    writeln!(file, "{}", block_height)?;
    writeln!(file, "{}", block_hash)?;
    Ok(())
}
