# Auto Display

This is a simple daemon that turns on when a device is connected via [UxPlay](https://github.com/FDH2/UxPlay) and off when it is disconnected.
This has been developed with a Raspberry Pi 5 in mind.
You might have to modify the screen code to make other devices work.

## Usage

```
-w, --watch-directory <WATCH_DIRECTORY>
        Directory to watch for the uxplay connection file [env: $HOME]
-i, --i2c-path <I2C_PATH>
        Path to the I2C device [env: $I2C_PATH]
    --i2c-on <I2C_ON>
        I2C address to turn the display on [default: 0x01]
    --i2c-off <I2C_OFF>
        I2C address to turn the display off [default: 0x04]
-t, --idle-timeout <IDLE_TIMEOUT>
        Time in seconds to wait before turning off the display [default: 900]
-h, --help
        Print help
```

You can use [`ddcutil`](https://github.com/rockowitz/ddcutil) to get the I2C device path of your display like this:

```sh
sudo ddcutil detect
```

You can get the values for on and off by running `ddcutil capabilities` and looking for `Feature: D6 (Power mode)`

```
   Feature: D6 (Power mode)
      Values:
         01: DPM: On,  DPMS: Off
         04: DPM: Off, DPMS: Off
```

## Installation

Ensure that the `-dacp` option is set for UxPlay.

```sh
sudo usermod -aG i2c $USER
# Reboot or logout/login here to apply the group change
cargo install --path .
mkdir -p ~/.config/systemd/user
# Adjust the path to the I2C device in auto-display.service
cp auto-display.service ~/.config/systemd/user/auto-display.service
systemctl --user enable --now auto-display.service
```
