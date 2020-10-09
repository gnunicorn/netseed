use structopt::StructOpt;

use async_std::{io, fs, path};
use async_std::net::TcpListener;
use async_std::prelude::*;

#[derive(StructOpt)]
struct Config{
    /// The port to listen on for incoming data
    /// '0'  will bind to a random system-decided port
    #[structopt(short, long, default_value="1337", env="PORT")]
    port: u16,
    /// The interface to bind to
    #[structopt(short, long, default_value="0.0.0.0", env="INTERFACE")]
    bind: String,
    /// Overwrite the file it already exists
    #[structopt(short, long)]
    force: bool,
    /// The files to write to in order of being received
    #[structopt(name="FILE")]
    paths: Vec<path::PathBuf>
}

#[async_std::main]
async fn main() -> Result<(), String>{
    let opt = Config::from_args();

    if opt.paths.len() == 0 {
        return Err("Please specify the files you want me to write to".to_owned());
    }

    let mut targets = opt.paths.iter();

    let listener = TcpListener::bind((opt.bind.as_str(), opt.port))
        .await.map_err(|e| format!("Error binding to addrs: {:}", e))?;
    println!("Listening on {}", listener.local_addr()
        .map_err(|e| format!("Fetching listening addr failed: {:}", e))?);

    let mut incoming = listener.incoming();

    let mut next_file = targets.next().expect("we checked there is at least one item");

    while let Some(stream) = incoming.next().await {
        let stream = stream.map_err(|e| format!("Error reading stream: {:}", e))?;

        if !opt.force && next_file.exists().await {
            return Err(format!("{:?} already exists, use '--force' if you want to overwrite", next_file.as_path()));
        }

        if let Some(parent_dir) = next_file.parent() {
            if !parent_dir.exists().await {
                fs::create_dir_all(parent_dir).await.map_err(|e| format!("Error creating directories: {:}", e))?;
            }
        }


        let (reader, _) = &mut (&stream, &stream);

        let mut writer = fs::File::create(next_file).await.map_err(|e| format!("Error creating file: {:}", e))?;
        io::copy(reader, &mut writer).await.map_err(|e| format!("Error writing stream to file: {:}", e))?;

        if let Some(next) = targets.next() {
            next_file = next
        } else {
            println!("Done writing all the files");
            break;
        }
    }

    Ok(())
}
