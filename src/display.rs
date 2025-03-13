use ddc::Ddc;
use std::time::Duration;
use zbus::{Connection, Proxy};

use crate::I2cConfig;

const GET_IDLE_TIME_METHOD: &str = "GetIdletime";
const POWER_MODE_FEATURE: u8 = 0xD6;
pub struct DisplayController<'a> {
    proxy: Proxy<'a>,
    i2c: I2cConfig,
}

impl<'a> DisplayController<'a> {
    pub async fn new(connection: &'a Connection, i2c: I2cConfig) -> anyhow::Result<Self> {
        let proxy = Proxy::new(
            &connection,
            "org.gnome.Mutter.IdleMonitor",
            "/org/gnome/Mutter/IdleMonitor/Core",
            "org.gnome.Mutter.IdleMonitor",
        )
        .await?;

        Ok(Self { proxy, i2c })
    }

    /// Returns the time the user has been idle for
    pub async fn idle_time(&self) -> anyhow::Result<Duration> {
        let message = self.proxy.call_method(GET_IDLE_TIME_METHOD, &()).await?;
        let value = message.body().deserialize::<u64>()?;
        let duration = Duration::from_millis(value);

        Ok(duration)
    }

    /// Returns whether the display is powered on or off
    pub async fn power(&self) -> anyhow::Result<bool> {
        let mut device = self.i2c.connect().await?;
        let (on, off) = self.i2c.values();

        let handle = tokio::task::spawn_blocking(move || {
            let result = device.get_vcp_feature(POWER_MODE_FEATURE)?;

            match result.value() {
                value if value == on => Ok(true),
                value if value == off => Ok(false),
                _ => Err(anyhow::anyhow!(
                    "Unexpected value for power status: {:x}",
                    result.value()
                )),
            }
        });

        handle.await?
    }

    /// Sets whether the display is powered on or off
    pub async fn set_power(&self, enabled: bool) -> anyhow::Result<()> {
        let mut device = self.i2c.connect().await?;
        let (on, off) = self.i2c.values();

        let handle = tokio::task::spawn_blocking(move || {
            device.set_vcp_feature(POWER_MODE_FEATURE, if enabled { on } else { off })?;

            Ok::<_, anyhow::Error>(())
        });

        handle.await??;
        Ok(())
    }
}
