use structopt::StructOpt;

use async_std::{io, fs, path};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;

#[derive(StructOpt)]
enum Command {
    /// Await files send over TCP and write them to local fs
    Await {
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
        /// The files to send in order
        #[structopt(name="FILE")]
        paths: Vec<path::PathBuf>
    },
    /// Send local file to remote netseeder
    Send {
        /// The port to send the for incoming data
        #[structopt(short, long, default_value="1337", env="PORT")]
        port: u16,
        /// The ip to send the data to
        #[structopt(short, long, default_value="127.0.0.1", env="INTERFACE")]
        ip: String,
        /// The files to write to in order of being received
        #[structopt(name="FILE")]
        paths: Vec<path::PathBuf>
    },
}

#[derive(StructOpt)]
struct Config{
    #[structopt(subcommand)]
    cmd: Command
}


#[async_std::main]
async fn main() -> Result<(), String> {
    let opt = Config::from_args();

    match opt.cmd {
        Command::Await {
            paths, bind,port,force,
        } => {
            wait_for_it(paths, bind, port, force).await
        },
        Command::Send {
            paths, ip,port,
        } => {
            send(paths, ip, port).await
        }
    }

}

async fn send(paths:Vec<path::PathBuf>, ip: String, port: u16) -> Result<(), String> {
    if paths.len() == 0 {
        return Err("Please specify the files you want me send".to_owned());
    }
    for f in paths {
        println!("Sending '{:}' ...", f.as_path().to_string_lossy());
        let mut reader = fs::File::open(f)
            .await.map_err(|e| format!("Could not open {}", e))?;

        // we are expected output to tell us, the forward is enabled
        let mut remote = TcpStream::connect((ip.as_str(), port))
            .await.map_err(|e| format!("Could not connect to remote: {:}", e))?;

        io::copy(&mut reader, &mut remote).await
            .map_err(|e| format!("Error writing stream to file: {:}", e))?;

        remote.shutdown(std::net::Shutdown::Write)
            .map_err(|e| format!("Could not close connection to remote: {:}", e))?;

    }
    println!("All files send");
    Ok(())
}

async fn wait_for_it(paths:Vec<path::PathBuf>, bind: String, port: u16, force: bool) -> Result<(), String> {
    if paths.len() == 0 {
        return Err("Please specify the files you want me to write to".to_owned());
    }

    println!("Expecting to receive data for: {}", paths.
        iter().map(|p|format!("{:}", p.as_path().to_string_lossy())).collect::<Vec<_>>().join("; "));

    let listener = TcpListener::bind((bind.as_str(), port))
        .await.map_err(|e| format!("Error binding to addrs: {:}", e))?;
    println!("Listening on {}", listener.local_addr()
        .map_err(|e| format!("Fetching listening addr failed: {:}", e))?);

    let mut incoming = listener.incoming();

    let mut targets = paths.iter();
    let mut next_file = targets.next().expect("we checked there is at least one item");
    println!("Waiting for '{:}' :", next_file.as_path().to_string_lossy());

    while let Some(stream) = incoming.next().await {

        let stream = stream.map_err(|e| format!("Error reading stream: {:}", e))?;

        if !force && next_file.exists().await {
            return Err(format!("{:?} already exists, use '--force' if you want to overwrite", next_file.as_path()));
        }

        if let Some(parent_dir) = next_file.parent() {
            if !parent_dir.exists().await {
                fs::create_dir_all(parent_dir).await
                    .map_err(|e| format!("Error creating directories: {:}", e))?;
            }
        }


        let (reader, _) = &mut (&stream, &stream);

        let mut writer = fs::File::create(next_file).await.map_err(|e| format!("Error creating file: {:}", e))?;
        io::copy(reader, &mut writer).await.map_err(|e| format!("Error writing stream to file: {:}", e))?;

        println!("✔️ received");

        if let Some(next) = targets.next() {
            next_file = next;
            println!("Waiting for '{:}' :", next_file.as_path().to_string_lossy());
        } else {
            println!("Done writing all the files");
            break;
        }
    }

    Ok(())
}
