use ddc_i2c::I2cDeviceDdc;
use std::path::PathBuf;

pub mod display;

#[derive(Debug, Copy, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct I2cConfig {
    pub on: u16,
    pub off: u16,
    pub path: PathBuf,
}

impl I2cConfig {
    pub async fn connect(&self) -> anyhow::Result<I2cDeviceDdc> {
        let path = self.path.clone();
        let handle = tokio::task::spawn_blocking(move || Ok(ddc_i2c::from_i2c_device(path)?));

        handle.await?
    }

    pub fn values(&self) -> (u16, u16) {
        (self.on, self.off)
    }
}
