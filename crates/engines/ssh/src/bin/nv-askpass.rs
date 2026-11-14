use std::env;
use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;

#[cfg(unix)]
use tokio::net::UnixStream;

#[tokio::main]
async fn main() {
    let pipe_name = match env::var("NV_ASKPASS_PIPE") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => std::process::exit(1),
    };
    let token = match env::var("NV_ASKPASS_TOKEN") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => std::process::exit(1),
    };

    #[cfg(windows)]
    {
        let mut client = match ClientOptions::new().open(&pipe_name) {
            Ok(c) => c,
            Err(_) => std::process::exit(1),
        };

        if client.write_all(format!("{}\n", token).as_bytes()).await.is_err() {
            std::process::exit(1);
        }

        let mut secret = Vec::new();
        if client.read_to_end(&mut secret).await.is_err() {
            std::process::exit(1);
        }

        if let Ok(mut stdout) = std::io::stdout().lock() {
            let _ = stdout.write_all(&secret);
            let _ = stdout.flush();
        }
    }

    #[cfg(unix)]
    {
        let mut stream = match UnixStream::connect(&pipe_name).await {
            Ok(s) => s,
            Err(_) => std::process::exit(1),
        };

        if stream.write_all(format!("{}\n", token).as_bytes()).await.is_err() {
            std::process::exit(1);
        }

        let mut secret = Vec::new();
        if stream.read_to_end(&mut secret).await.is_err() {
            std::process::exit(1);
        }

        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(&secret);
        let _ = stdout.flush();
    }
}
