use std::io;

#[cfg(feature = "unblock")]
use std::{env, fs};

#[cfg(feature = "unblock")]
use futures::stream::StreamExt;

fn main() -> io::Result<()> {
    #[cfg(feature = "unblock")]
    {
        let path = env::args().nth(1).unwrap_or_else(|| ".".to_string());

        easy_async::block_on(async {
            let mut dir = easy_async::Unblock::new(fs::read_dir(path)?);

            while let Some(item) = dir.next().await {
                println!("{}", item?.file_name().to_string_lossy());
            }

            Ok(())
        })
    }

    #[cfg(not(feature = "unblock"))]
    Ok(())
}
