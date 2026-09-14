#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Metadata {
    pub name: &'static str,
    pub family: &'static str,
    // pub memory: &'static [MemoryRegion],
    pub peripherals: &'static [Peripheral],
    pub pins: &'static [Pin],
    // pub nvic_priority_bits: Option<u8>,
    pub interrupts: &'static [Interrupt],
    pub interrupt_groups: &'static [InterruptGroup],
    pub dma_channels: &'static [DmaChannel],
    pub adc_memctl: u8,
    pub adc_vrsel: u8,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Peripheral {
    pub name: &'static str,
    pub kind: &'static str,
    pub version: Option<&'static str>,
    pub pins: &'static [PeripheralPin],
    pub power_domain: PowerDomain,
    pub sys_fentries: Option<usize>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Pin {
    pub pin: &'static str,
    pub pincm: u8,

    /// Whether the pin has wakeup logic, and can therefore wake the device from SHUTDOWN.
    ///
    /// This is not the same as the pin being able to wake the device at all: `FASTWAKE` works on
    /// any GPIO pin, but only down to STANDBY.
    ///
    /// `None` when the vendor data does not describe wakeup logic for this chip, which is not the
    /// same as `Some(false)`. **That is the common case**: sysconfig carries the attribute on only
    /// 7 of the 18 families, so `None` on every pin is the answer for 30 of the 43 part numbers.
    /// The seven are c110x, c1105_c1106, g151x, g351x, g518x, l112x and l211x.
    ///
    /// Do not fall back to [`Pin::structure`] where this is `None`. SLAU846 Table 8-1 maps structure
    /// to wake capability and is wrong on mspm0c110x and msps003fx, whose open-drain pins have no
    /// wakeup logic. On mspm0c110x this field says so — every pin is `Some(false)`, the two
    /// open-drain pins included. On msps003fx it is `None`, so nothing contradicts the table there.
    pub wakeup: Option<bool>,

    /// Which IO structure the pin is built from, and so which of its PINCM fields do anything.
    pub structure: IoStructure,
}

/// The IO structure a pin is built from, which decides what its PINCM fields do.
///
/// The IOMUX register is the same for every pin, so a field a pin's structure does not implement
/// is written, read back, and ignored. What each structure implements:
///
/// | structure | `INV` | `DRV` | `HYSTEN` | `PIPU` | `PIPD` | wake |
/// |---|---|---|---|---|---|---|
/// | [`Standard`](IoStructure::Standard), [`StandardLowLeakage`](IoStructure::StandardLowLeakage) | yes | | | yes | yes | |
/// | [`StandardWithWake`](IoStructure::StandardWithWake) | yes | | | yes | yes | yes |
/// | [`HighDrive`](IoStructure::HighDrive) | yes | yes | | yes | yes | yes |
/// | [`HighSpeed`](IoStructure::HighSpeed) | yes | yes | | yes | yes | |
/// | [`OpenDrain`](IoStructure::OpenDrain) | yes | | yes | | yes | yes |
///
/// The table is SLAU846 Table 8-1, and TI's own `GPIOPin.syscfg.js` gates its options by exactly
/// these rules. Three caveats before treating it as complete:
///
/// - **Wake is not derivable from the structure.** On mspm0c110x and msps003fx the open-drain
///   pins have no wakeup logic — sysconfig marks `io_wakeup` false on them, and the C1104
///   datasheet's feature table has no wakeup column at all. Use [`Pin::wakeup`].
/// - **The per-device feature tables are not reliable in either direction.** The MSPM0G3519's
///   omits the open-drain row although its own pin table gives PA0 and PA1 that structure; the
///   MSPM0L2117's carries two low-drive rows although no pin on the device is low-drive; the
///   MSPM0L2117's also leaves high-drive's drive-strength cell empty against every other
///   datasheet, the TRM and TI's own tool; and the MSPM0H3216's marks no structure as having a
///   pulldown. Per-pin data is the reliable part.
/// - **Not every device has every structure**, and no device has all of them.
///
/// The source is sysconfig's per-pin `io_type`. The datasheets' per-pin tables corroborate it on
/// 723 of the 847 pins, across all 18 families, with no disagreement on any of them; the shortfall
/// is rows the table extraction did not recover, not pins the two sources describe differently.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum IoStructure {
    /// Standard drive (`SDIO`).
    Standard,

    /// Standard drive, low leakage (`SDL` in sysconfig, which TI's tool calls "Low-leakage
    /// Standard"). The datasheets' pin tables print it as plain standard drive, and every rule in
    /// TI's tool treats the two identically, so the difference is leakage current rather than
    /// anything the IOMUX can express.
    ///
    /// One pin per family on the older G and L families — PA2 everywhere it appears — and every
    /// pin of msps003fx.
    StandardLowLeakage,

    /// Standard drive with wakeup logic (`SDIO` with wake).
    StandardWithWake,

    /// High drive (`HDIO`), the 20mA output.
    HighDrive,

    /// High speed (`HSIO`).
    HighSpeed,

    /// 5V-tolerant open drain (`ODIO`). The only structure with hysteresis control, and the only
    /// one with no pullup: `PIPU` on one of these pins does nothing.
    OpenDrain,

    /// A USB 2.0 full-speed pin (`USBIO`), on mspm0g518x only. Powered from `VUSB33` rather than
    /// `VDD`, and treated as standard drive by TI's tool.
    Usb,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PeripheralPin {
    pub pin: &'static str,
    pub signal: &'static str,
    pub pf: Option<u8>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PowerDomain {
    /// "low speed" power domain. This power domain is powered in RUN, SLEEP, STOP and STANDBY modes.
    Pd0,

    /// "high performance" power domain. This power domain is powered in RUN and SLEEP modes.
    Pd1,

    /// PDB backup power domain. This is usually powered by VBAT.
    ///
    /// Not available on every chip.
    Backup,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Interrupt {
    pub name: &'static str,
    pub number: u32,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct InterruptGroup {
    pub name: &'static str,
    pub number: u32,
    pub interrupts: &'static [GroupInterrupt],
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct GroupInterrupt {
    pub name: &'static str,
    pub number: u32,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct DmaChannel {
    /// The number of the dma channel.
    pub number: u8,

    /// Whether this is a full or basic dma channel.
    pub full: bool,
}
