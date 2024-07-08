use std::sync::Arc;
use std::time::Duration;

use cdk::amount::{Amount, SplitTarget};
use cdk::cdk_database::WalletMemoryDatabase;
use cdk::nuts::{CurrencyUnit, MintQuoteState};
use cdk::wallet::Wallet;
use rand::Rng;
use tokio::task::JoinSet;
use tokio::time::{sleep, Instant};

const SECS_TO_RUN: u64 = 60;
const WALLET_COUNT: u64 = 10;

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

    let mut handles = JoinSet::new();

    let all_start = Instant::now();

    for i in 0..WALLET_COUNT {
        let starting_token = wallet
            .send(Amount::from(100), None, None, &SplitTarget::default())
            .await
            .unwrap();

        println!("Starting wallet {}", i);

        handles.spawn(wallet_swap(
            mint_url,
            CurrencyUnit::Sat,
            starting_token.clone(),
        ));
    }

    let mut total_transaction = 0;
    while let Some(Ok(result)) = handles.join_next().await {
        total_transaction += result;
        println!("Wallet completed {} transaction", result);
        println!("tps: {}", result / SECS_TO_RUN);
    }

    println!("Total transaction: {}", total_transaction);

    println!(
        "avg tps: {}",
        total_transaction / all_start.elapsed().as_secs()
    );

    Ok(())
}

async fn wallet_swap(mint_url: &str, unit: CurrencyUnit, starting_token: String) -> u64 {
    // Create a new Tokio runtime for this thread
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

    while start.elapsed() < Duration::from_secs(SECS_TO_RUN) {
        let token = wallet
            .send(Amount::from(3), None, None, &SplitTarget::default())
            .await
            .unwrap();

        wallet
            .receive(&token.to_string(), &SplitTarget::default(), &[], &[])
            .await
            .unwrap();

        transaction_count += 1;
    }

    transaction_count
}
