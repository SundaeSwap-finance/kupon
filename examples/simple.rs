use kupon::{Builder, Client, DatumHash, MatchOptions};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kupon: Client = Builder::with_endpoint("http://localhost:1442")
        .with_retries(3)
        .build()?;

    let options: MatchOptions = MatchOptions::default()
        .only_unspent()
        .address("addr1w9qzpelu9hn45pefc0xr4ac4kdxeswq7pndul2vuj59u8tqaxdznu");

    let matches = kupon.matches(&options).await?;
    for matc in matches {
        println!("TX {}:", matc.transaction_id);
        println!("\tlovelace: {}", matc.value.coins);
        for (asset_id, value) in matc.value.assets {
            println!("\t{:?}: {}", asset_id, value);
        }

        if let Some(DatumHash { hash, .. }) = &matc.datum {
            if let Some(datum) = kupon.datum(hash).await? {
                println!("\tdatum: {}", datum);
            }
        }
    }
    Ok(())
}
