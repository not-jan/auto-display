use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use auto_display::{ConnectionState, I2cConfig, display::DisplayController};
use clap::Parser;
use futures::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use zbus::Connection;

const UXPLAY_CONNECTION_FILE: &str = ".uxplay.dacp";

#[derive(Debug, Clone, clap::Parser)]
struct Args {
    /// Directory to watch for the uxplay connection file
    #[clap(short, long, env = "HOME")]
    watch_directory: PathBuf,

    /// Path to the I2C device
    #[clap(short, long, env = "I2C_PATH", default_value = "/dev/i2c-12")]
    i2c_path: PathBuf,

    /// I2C address to turn the display on
    #[clap(long, default_value_t = 0x01)]
    i2c_on: u16,

    /// I2C address to turn the display off
    #[clap(long, default_value_t = 0x04)]
    i2c_off: u16,

    /// Time in seconds to wait before turning off the display
    #[clap(short = 't', long, default_value = "900")]
    idle_timeout: u64,
}

async fn watch_directory(
    path: impl AsRef<Path>,
) -> anyhow::Result<mpsc::Receiver<Result<ConnectionState, std::io::Error>>> {
    let inotify = Inotify::init()?;
    inotify
        .watches()
        .add(path, WatchMask::CREATE | WatchMask::DELETE)
        .expect("Failed to add inotify watch");

    let (tx, rx) = mpsc::channel::<Result<ConnectionState, std::io::Error>>(16);

    tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut stream = inotify
            .into_event_stream(&mut buffer)?
            .filter_map(async |event| match event {
                Ok(event) => match event.name.as_ref().and_then(|name| name.to_str()) {
                    Some(name) if name == UXPLAY_CONNECTION_FILE => {
                        if event.mask.contains(EventMask::CREATE) {
                            Some(Ok(ConnectionState::Connected))
                        } else if event.mask.contains(EventMask::DELETE) {
                            Some(Ok(ConnectionState::Disconnected))
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                Err(e) => Some(Err(e)),
            })
            .boxed();

        while let Some(event) = stream.next().await {
            match tx.send(event).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("Failed to send event: {:?}", e);
                    break;
                }
            }
        }

        Ok::<_, std::io::Error>(())
    });

    Ok(rx)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?;
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let args = Args::parse();

    tracing::info!("Watching directory: {:?}", args.watch_directory);

    let connection = Connection::session().await?;
    let display = DisplayController::new(
        &connection,
        I2cConfig {
            on: args.i2c_on,
            off: args.i2c_off,
            path: args.i2c_path.clone(),
        },
    )
    .await?;

    let rx = watch_directory(&args.watch_directory).await?;
    let mut rx = ReceiverStream::new(rx);

    let timeout = Duration::from_secs(args.idle_timeout);

    while let Some(result) = rx.next().await {
        match result {
            Ok(connection_state) => {
                let idle_time = display.idle_time().await?;
                tracing::debug!("Idle time: {:?}", idle_time);
                let is_powered_on = display.power().await?;

                match (is_powered_on, connection_state) {
                    (false, ConnectionState::Connected) => {
                        tracing::debug!("User connected. Turning display on.");
                        display.set_power(true).await?;
                    }
                    (true, ConnectionState::Disconnected) if idle_time > timeout => {
                        tracing::debug!("User disconnected. Turning display off.");
                        display.set_power(false).await?;
                    }
                    _ => {}
                }
            }

            Err(error) => {
                tracing::error!("Error while watching directory: {:?}", error);
                break;
            }
        }
    }

    Ok(())
}
