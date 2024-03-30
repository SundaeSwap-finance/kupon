use kupon::{Builder, Client, MatchOptions};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kupon: Client = Builder::with_endpoint("http://localhost:1442").build()?;

    let options: MatchOptions = MatchOptions::default()
        .only_unspent()
        .address("addr_test1qrkua4yzegsu8g0xh0mkcc4uufdh2vnvfw2rhxmc6f30s6jdp276p7j8023vmum9wu8gp7q54f3rjke45j0klk3pmwvsttajw2");

    let matches = kupon.matches(&options).await?;
    for m in matches {
        println!("{:?}", m);
    }
    Ok(())
}
