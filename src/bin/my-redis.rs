

use mini_redis::{client,Result};


#[tokio::main]
async fn main()->Result<()>{
    let mut client=client::connect("localhost:6380").await?;
    client.set("hello","world".into()).await?;
    let result=client.get("hello").await?;
    println!("got {result:?}");
    Ok(())
}