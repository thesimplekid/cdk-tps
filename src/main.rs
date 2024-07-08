use std::sync::Arc;
use std::thread;
use std::time::Duration;

use cdk::amount::{Amount, SplitTarget};
use cdk::cdk_database::WalletMemoryDatabase;
use cdk::nuts::{CurrencyUnit, MintQuoteState};
use cdk::wallet::Wallet;
use rand::Rng;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Instant};

const SECS_TO_RUN: u64 = 60;
const WALLET_COUNT: u64 = 16;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let seed = rand::thread_rng().gen::<[u8; 32]>();

    let mint_url = "https://mint.thesimplekid.dev";
    //let mint_url = "http://127.0.0.1:8085";
    let unit = CurrencyUnit::Sat;
    let amount = Amount::from(100000);

    let localstore = WalletMemoryDatabase::default();

    let wallet = Wallet::new(mint_url, unit, Arc::new(localstore), &seed);

    let quote = wallet.mint_quote(amount).await.unwrap();
    println!("Pay request: {}", quote.request);

    loop {
        let status = wallet.mint_quote_state(&quote.id).await.unwrap();

        if status.state == MintQuoteState::Paid {
            break;
        }

        println!("Quote state: {}", status.state);

        sleep(Duration::from_secs(5)).await;
    }
    let receive_amount = wallet
        .mint(&quote.id, SplitTarget::default(), None)
        .await
        .unwrap();

    println!("Minted: {}", receive_amount);

    let mut handles = vec![];

    let all_satart = Instant::now();

    for i in 0..WALLET_COUNT {
        let b = wallet.total_balance().await?;

        println!("{}", b);
        let starting_token = wallet
            .send(Amount::from(100), None, None, &SplitTarget::default())
            .await
            .unwrap();
        // Clone data to move into the thread

        println!("Starting wallet {}", i);

        // Spawn a new OS thread for each wallet
        let handle = thread::spawn(move || {
            // Create a new RNG for this thread

            // Create a new Tokio runtime for this thread
            let rt = Runtime::new().unwrap();

            rt.block_on(async move {
                let seed = rand::thread_rng().gen::<[u8; 32]>();
                let wallet = Wallet::new(
                    &mint_url,
                    unit,
                    Arc::new(WalletMemoryDatabase::default()),
                    &seed,
                );
                wallet
                    .receive(&starting_token, &SplitTarget::default(), &[], &[])
                    .await
                    .unwrap();

                let mut transaction_count = 0;
                let start = Instant::now();

                println!("Starting swapping on wallet {}", i);

                while start.elapsed() < Duration::from_secs(SECS_TO_RUN) {
                    let mut rng = rand::thread_rng();
                    let random_number: u8 = rng.gen_range(1..=20);
                    let token = wallet
                        .send(
                            Amount::from(random_number as u64),
                            None,
                            None,
                            &SplitTarget::default(),
                        )
                        .await
                        .unwrap();

                    wallet
                        .receive(&token.to_string(), &SplitTarget::default(), &[], &[])
                        .await
                        .unwrap();

                    transaction_count += 1;
                }

                transaction_count
            })
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();

    let mut total_transaction = 0;
    // Process results
    for (index, result) in results.into_iter().enumerate() {
        total_transaction += result;
        println!("Wallet {} completed {} transaction", index, result);
        println!("tps: {}", result / SECS_TO_RUN);
    }

    println!("Total transaction: {}", total_transaction);

    println!(
        "avg tps: {}",
        total_transaction / all_satart.elapsed().as_secs()
    );

    Ok(())
}
