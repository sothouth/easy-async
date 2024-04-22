use std::{env, fs, io};

use futures::prelude::*;

use easy_async::block_on;
use easy_async::Unblock;

fn main() -> io::Result<()> {
    let path = env::args().nth(1).unwrap_or_else(|| ".".to_string());

    block_on(async {
        let mut dir = Unblock::new(fs::read_dir(path)?);

        while let Some(item) = dir.next().await {
            println!("{}", item?.file_name().to_string_lossy());
        }

        Ok(())
    })
}
