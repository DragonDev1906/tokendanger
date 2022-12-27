mod dynchunkiter;
mod erc165;
mod erc721;
mod storage;
mod types;

use std::{env, ops::Range, io};
use async_recursion::async_recursion;
use dynchunkiter::DynChunkIter;
use hex_literal::hex;
use storage::{memory::MemoryStorage, Storage};
use web3::{
    types::{FilterBuilder, Log, H256, U256, U64},
    Transport, Web3,
};
use types::ContractType;

const PERSISTENCE_PATH: &'static str = "./contracts.json";
const TOO_MANY_RESULTS_ERRCODE: i64 = -32005;
const ERC721_TRANSFER_EVENT: H256 = H256(hex!(
    "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
));

#[tokio::main]
async fn main() {
    let infura_key = env::var("INFURA_KEY").expect("environment variable INFURA_KEY not found.");
    let url = format!("https://mainnet.infura.io/v3/{}", infura_key);

    let transport = web3::transports::Http::new(&url).expect("could not connec to infura");
    let w3 = Web3::new(transport);

    // Go-Ethereum allows up to 10_000 return values, to avoid too many errors
    // we instead target a smaller amount.
    let mut iter = DynChunkIter::new(16249100, 10, 8_000);
    while let Some(chunk) = iter.next() {
        // Retry as long as the range is too large
        let range = chunk.start.into()..chunk.end.into();
        match task(&w3, range).await {
            Ok(amount) => {
                println!("Task Amount: {}", amount);
                iter.update_chunk_size(amount);
            },
            Err(e) => panic!("Task returned error: {:?}", e),
        }
    }
}

#[async_recursion(?Send)]
async fn task<T: Transport>(w3: &Web3<T>, range: Range<U64>) -> Result<usize, TaskError> {
    // ERC721_TRANSFER_EVENT also catches ERC20 Transfers, as the only
    // difference is that the third param is indexed.

    // Request all relevant events in this range.
    let topic1 = vec![ERC721_TRANSFER_EVENT];
    let filter = FilterBuilder::default()
        .from_block(range.start.into())
        .to_block(range.end.into())
        .topics(Some(topic1), None, None, None)
        .build();
    let logs = match w3.eth().logs(filter).await {
        Ok(v) => v,
        Err(web3::Error::Rpc(e)) if e.code.code() == TOO_MANY_RESULTS_ERRCODE => {
            // Split range in two parts and call recursively (try to avoid this
            // case as it results in a "useless" call that just returns the too
            // many results error).
            let middle = (range.start + range.end) / 2;
            let lower_amount = task(w3, range.start..middle).await?;
            let upper_amount = task(w3, middle..range.end).await?;
            return Ok(lower_amount + upper_amount);
        }
        Err(e) => return Err(e.into()),
    };
    let return_amount = logs.len();

    // Process the results we got.
    process_logs(w3, logs).await?;

    Ok(return_amount)
}

async fn process_logs<T: Transport>(w3: &Web3<T>, logs: Vec<Log>) -> Result<(), TaskError> {
    let mut storage = MemoryStorage::new(PERSISTENCE_PATH)?;

    // TODO: Don't stop after 40 entries, this is just to prevent sending too
    // many requests while testing.
    for log in &logs[..40] {
        let contract_type = match storage.get_contract_type(log.address) {
            Some(t) => t,
            None => {
                // We don't have it yet. This should always write to storage,
                // but just in case we do modify it in paralell in the future
                // this throws away the just requested contract type (earliest
                // write first). Both return the same value unless the smart
                // contract has weird behavior.
                let t = contract_type(w3, &log).await?;
                storage.store_contract_type(log.address, t)
            },
        };

        match contract_type {
            ContractType::Unknown => {}
            ContractType::UnknownERC165 => {}
            ContractType::ERC721 { metadata, .. } => {
                // Only relevant if this isn't a token burn. It may still be burned in
                // the future (e.g. if we're processing old blocks), which means getting the
                // metadata will fail, there is nothing we can do about that unless we have
                // an archive node.
                if metadata && log.topics[2].0 != [0u8; 32] {
                    match erc721::metadata_token_uri(
                        w3,
                        log.address,
                        U256::from_big_endian(&log.topics[3].0),
                    )
                    .await
                    {
                        Ok(uri) => {
                            println!("URI: {}", uri);
                        }
                        Err(erc721::Error::Contract(web3::contract::Error::Api(
                            web3::Error::Rpc(e),
                        ))) if e.code.code() == 3 => {
                            println!("tokenUri reverted: {:?}", log);
                        }
                        Err(e) => panic!("Unexpected error: {:?}", e),
                    };
                }
            }
            ContractType::MaybeERC20 => {}
        }

        // println!("Contract type: {:?}", contract_type);
    }

    // Store contact type info back to file
    storage.persist().unwrap();

    Ok(())
}

async fn contract_type<T: Transport>(
    w3: &Web3<T>,
    log: &Log,
) -> Result<ContractType, erc165::Error> {
    let addr = log.address;
    println!("Request type for {}", addr);
    if !erc165::is_eip165(w3, addr).await? {
        return if log.topics.len() == 3 && log.data.0.len() == 32 {
            Ok(ContractType::MaybeERC20)
        } else {
            Ok(ContractType::Unknown)
        };
    }

    if erc165::is_erc721_unchecked(w3, addr).await? {
        Ok(ContractType::ERC721 {
            metadata: erc165::is_erc721metadata_unchecked(w3, addr).await?,
            enumerable: erc165::is_erc721enumerable_unchecked(w3, addr).await?,
        })
    } else {
        Ok(ContractType::UnknownERC165)
    }
}

#[derive(Debug)]
enum TaskError {
    Web3(web3::Error),
    Erc165(erc165::Error),
    IO(io::Error),
}

impl From<web3::Error> for TaskError {
    fn from(e: web3::Error) -> Self {
        TaskError::Web3(e)
    }
}
impl From<erc165::Error> for TaskError {
    fn from(e: erc165::Error) -> Self {
        TaskError::Erc165(e)
    }
}
impl From<io::Error> for TaskError {
    fn from(e: io::Error) -> Self {
        TaskError::IO(e)
    }
}
