//! # Use ws2812 leds via spi
//!
//! - For usage with `smart-leds`
//! - Implements the `SmartLedsWrite` trait
//!
//! Needs a type implementing the `spi::FullDuplex` trait.
//!
//! The spi peripheral should run at 3MHz for WS2812 LEDs, or 4MHz for SK6812w
//! LEDs.

#![no_std]

extern crate embedded_hal as hal;

pub mod prerendered;

use hal::spi::{FullDuplex, Mode, Phase, Polarity};

use smart_leds_trait::{SmartLedsWrite, RGB8, RGBW};

use nb;
use nb::block;

/// SPI mode that can be used for this crate
///
/// Provided for convenience
/// Doesn't really matter
pub const MODE: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

/// The internal communication layer implementation.
struct CommLayer<SPI> {
    spi: SPI,
}

impl<SPI, E> CommLayer<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    /// The SPI bus should run with 3 Mhz, otherwise this won't work.
    ///
    /// You may need to look at the datasheet and your own hal to verify this.
    ///
    /// Please ensure that the mcu is pretty fast, otherwise weird timing
    /// issues will occur
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Write a single byte for ws2812 devices
    fn write_byte(&mut self, mut data: u8) -> Result<(), E> {
        let mut serial_bits: u32 = 0;
        for _ in 0..3 {
            let bit = data & 0x80;
            let pattern = if bit == 0x80 { 0b110 } else { 0b100 };
            serial_bits = pattern | (serial_bits << 3);
            data <<= 1;
        }
        block!(self.spi.send((serial_bits >> 1) as u8))?;
        // Split this up to have a bit more lenient timing
        for _ in 3..8 {
            let bit = data & 0x80;
            let pattern = if bit == 0x80 { 0b110 } else { 0b100 };
            serial_bits = pattern | (serial_bits << 3);
            data <<= 1;
        }
        // Some implementations (stm32f0xx-hal) want a matching read
        // We don't want to block so we just hope it's ok this way
        self.spi.read().ok();
        block!(self.spi.send((serial_bits >> 8) as u8))?;
        self.spi.read().ok();
        block!(self.spi.send(serial_bits as u8))?;
        self.spi.read().ok();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), E> {
        for _ in 0..20 {
            block!(self.spi.send(0))?;
            self.spi.read().ok();
        }
        Ok(())
    }
}

/// Driver for strings of Ws2812 LEDs. This driver expects the SPI bus to be
/// running at ~3MHz.
pub struct Ws2812<SPI> {
    comms: CommLayer<SPI>,
}

impl<SPI, E> Ws2812<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    /// Create a smart led strip driver from the provided SPI peripheral. The
    /// peripheral should be running at 3 MHz.
    pub fn new(spi: SPI) -> Self {
        Self {
            comms: CommLayer::new(spi),
        }
    }
}

/// Driver for strings of SK6812-W LEDs. This driver expects the SPI bus to be
/// running at ~4MHz.
pub struct Sk6812w<SPI> {
    comms: CommLayer<SPI>,
}

impl<SPI, E> Sk6812w<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    /// Create a smart led strip driver from the provided SPI peripheral. The
    /// peripheral should be running at 4 MHz.
    pub fn new(spi: SPI) -> Self {
        Self {
            comms: CommLayer::new(spi),
        }
    }
}

impl<SPI, E> SmartLedsWrite for Sk6812w<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    type Error = E;
    type Color = RGBW<u8, u8>;
    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), E>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>,
    {
        if cfg!(feature = "mosi_idle_high") {
            self.comms.flush()?;
        }

        for item in iterator {
            let item = item.into();
            self.comms.write_byte(item.g)?;
            self.comms.write_byte(item.r)?;
            self.comms.write_byte(item.b)?;
            self.comms.write_byte(item.a.0)?;
        }
        self.comms.flush()?;
        Ok(())
    }
}

impl<SPI, E> SmartLedsWrite for Ws2812<SPI>
where
    SPI: FullDuplex<u8, Error = E>,
{
    type Error = E;
    type Color = RGB8;
    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), E>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>,
    {
        if cfg!(feature = "mosi_idle_high") {
            self.comms.flush()?;
        }

        for item in iterator {
            let item = item.into();
            self.comms.write_byte(item.g)?;
            self.comms.write_byte(item.r)?;
            self.comms.write_byte(item.b)?;
        }
        self.comms.flush()?;
        Ok(())
    }
}
