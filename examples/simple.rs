use kupon::{Builder, Client, MatchOptions};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kupon: Client = Builder::with_endpoint("http://localhost:1442").build()?;

    let options: MatchOptions = MatchOptions::default()
        .only_unspent()
        .address("addr_test1wpesulg5dtt5y73r4zzay9qmy3wnlrxdg944xg4rzuvewls7nrsf0");

    let matches = kupon.matches(&options).await?;
    for m in matches {
        println!("{:?}", m);
    }
    Ok(())
}
