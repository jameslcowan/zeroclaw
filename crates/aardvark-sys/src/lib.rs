//! Stub bindings for the Total Phase Aardvark I2C/SPI/GPIO USB adapter.
//!
//! This crate exposes a safe Rust API over the Aardvark C library.
//!
//! # Current state — Stub
//!
//! The Total Phase SDK (`aardvark.h` + `aardvark.so`) is not yet committed.
//! All [`AardvarkHandle`] methods currently return
//! [`Err(AardvarkError::NotFound)`](AardvarkError::NotFound) at runtime.
//! Build and link succeed without the SDK.
//!
//! # Enabling real hardware
//!
//! 1. Download the Total Phase Aardvark Software API from
//!    <https://www.totalphase.com/products/aardvark-software-api/>
//! 2. Copy the SDK files into this crate:
//!    - `aardvark.h`  → `crates/aardvark-sys/vendor/aardvark.h`
//!    - `aardvark.so` → `crates/aardvark-sys/vendor/aardvark.so`  (Linux / macOS)
//!    - `aardvark.dll`→ `crates/aardvark-sys/vendor/aardvark.dll` (Windows)
//! 3. In `Cargo.toml` add `bindgen = "0.69"` to `[build-dependencies]`
//!    and `libc = "0.2"` to `[dependencies]`.
//! 4. Uncomment the bindgen block in `build.rs`.
//! 5. Replace the stub method bodies below with FFI calls via `mod bindings`.
//!
//! This crate is the **only** place in ZeroClaw where `unsafe` is permitted.
//! The rest of the workspace stays `#![forbid(unsafe_code)]`.

use thiserror::Error;

// When the SDK is present and bindgen has run, un-comment:
// mod bindings;

/// Errors returned by Aardvark hardware operations.
#[derive(Debug, Error)]
pub enum AardvarkError {
    /// No Aardvark adapter found — adapter not plugged in or SDK absent.
    #[error("Aardvark adapter not found — is it plugged in?")]
    NotFound,
    /// `aa_open` returned a non-positive handle.
    #[error("Aardvark open failed (code {0})")]
    OpenFailed(i32),
    /// `aa_i2c_write` returned a negative status code.
    #[error("I2C write failed (code {0})")]
    I2cWriteFailed(i32),
    /// `aa_i2c_read` returned a negative status code.
    #[error("I2C read failed (code {0})")]
    I2cReadFailed(i32),
    /// `aa_spi_write` returned a negative status code.
    #[error("SPI transfer failed (code {0})")]
    SpiTransferFailed(i32),
    /// GPIO operation returned a negative status code.
    #[error("GPIO error (code {0})")]
    GpioError(i32),
}

/// Convenience `Result` alias for this crate.
pub type Result<T> = std::result::Result<T, AardvarkError>;

/// Safe RAII handle over the Aardvark C library handle.
///
/// Automatically closes the adapter on `Drop`.
///
/// **Usage pattern:** open a fresh handle per command and let it drop at the
/// end of each operation — the same lazy-open / eager-close strategy used by
/// [`HardwareSerialTransport`](../zeroclaw/hardware/serial/struct.HardwareSerialTransport.html)
/// for serial devices.
pub struct AardvarkHandle {
    _port: i32,
}

impl AardvarkHandle {
    // ── Lifecycle ─────────────────────────────────────────────────────────────

    /// Open the first available Aardvark adapter (port 0).
    ///
    /// Equivalent to `aa_open(0)`.
    pub fn open() -> Result<Self> {
        // Stub: SDK not linked.
        Err(AardvarkError::NotFound)
    }

    /// Open a specific Aardvark adapter by port index.
    ///
    /// Equivalent to `aa_open(port)`.
    pub fn open_port(port: i32) -> Result<Self> {
        // Stub: SDK not linked.
        let _ = port;
        Err(AardvarkError::NotFound)
    }

    /// Return the port numbers of all connected Aardvark adapters.
    ///
    /// Equivalent to `aa_find_devices(16, ports)`.
    /// Returns an empty `Vec` when the SDK is not linked.
    pub fn find_devices() -> Vec<u16> {
        // Stub: no hardware available.
        Vec::new()
    }

    // ── I2C ───────────────────────────────────────────────────────────────────

    /// Enable I2C mode and set the bitrate.
    ///
    /// Configures the adapter for I2C-only mode, sets pullups, and applies
    /// the requested bitrate.
    pub fn i2c_enable(&self, _bitrate_khz: u32) -> Result<()> {
        Err(AardvarkError::NotFound)
    }

    /// Write `data` bytes to the I2C device at `addr`.
    pub fn i2c_write(&self, _addr: u8, _data: &[u8]) -> Result<()> {
        Err(AardvarkError::NotFound)
    }

    /// Read `len` bytes from the I2C device at `addr`.
    pub fn i2c_read(&self, _addr: u8, _len: usize) -> Result<Vec<u8>> {
        Err(AardvarkError::NotFound)
    }

    /// Write then read — the standard I2C register-read pattern.
    ///
    /// Sends `write_data` to `addr` (sets the register pointer), then reads
    /// `read_len` bytes back from the same address.
    pub fn i2c_write_read(&self, addr: u8, write_data: &[u8], read_len: usize) -> Result<Vec<u8>> {
        self.i2c_write(addr, write_data)?;
        self.i2c_read(addr, read_len)
    }

    /// Scan the I2C bus for responding devices.
    ///
    /// Probes addresses `0x08–0x77` with a 1-byte read.  Returns the list
    /// of addresses that ACK.  Returns an empty `Vec` in stub mode.
    pub fn i2c_scan(&self) -> Vec<u8> {
        // Stub: no hardware.
        Vec::new()
    }

    // ── SPI ───────────────────────────────────────────────────────────────────

    /// Enable SPI mode and set the bitrate.
    pub fn spi_enable(&self, _bitrate_khz: u32) -> Result<()> {
        Err(AardvarkError::NotFound)
    }

    /// Perform a full-duplex SPI transfer.
    ///
    /// Sends the bytes in `send` and returns the simultaneously received bytes.
    /// The returned `Vec` has the same length as `send`.
    pub fn spi_transfer(&self, _send: &[u8]) -> Result<Vec<u8>> {
        Err(AardvarkError::NotFound)
    }

    // ── GPIO ──────────────────────────────────────────────────────────────────

    /// Set GPIO pin directions and output values.
    ///
    /// `direction` is a bitmask: `1` = output, `0` = input.
    /// `value` is a bitmask of the output states.
    pub fn gpio_set(&self, _direction: u8, _value: u8) -> Result<()> {
        Err(AardvarkError::NotFound)
    }

    /// Read the current GPIO pin states.
    ///
    /// Returns a bitmask of the current pin levels.
    pub fn gpio_get(&self) -> Result<u8> {
        Err(AardvarkError::NotFound)
    }
}

impl Drop for AardvarkHandle {
    fn drop(&mut self) {
        // Stub: nothing to close.
        // Real: unsafe { bindings::aa_close(self._port); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_devices_returns_empty_when_sdk_absent() {
        assert!(AardvarkHandle::find_devices().is_empty());
    }

    #[test]
    fn open_returns_not_found_when_sdk_absent() {
        assert!(matches!(
            AardvarkHandle::open(),
            Err(AardvarkError::NotFound)
        ));
    }

    #[test]
    fn open_port_returns_not_found_when_sdk_absent() {
        assert!(matches!(
            AardvarkHandle::open_port(0),
            Err(AardvarkError::NotFound)
        ));
    }

    #[test]
    fn error_display_messages_are_human_readable() {
        assert!(AardvarkError::NotFound.to_string().contains("not found"));
        assert!(AardvarkError::OpenFailed(-1).to_string().contains("-1"));
        assert!(AardvarkError::I2cWriteFailed(-3)
            .to_string()
            .contains("I2C write"));
        assert!(AardvarkError::SpiTransferFailed(-2)
            .to_string()
            .contains("SPI"));
    }
}
