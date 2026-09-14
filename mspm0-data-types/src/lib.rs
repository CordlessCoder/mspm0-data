use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Chip {
    /// The chip name.
    ///
    /// This shall not contain any placeholders and be a full chip name like mspm0g3507.
    pub name: String,

    /// The device family.
    ///
    /// Usually this is a value like `mspm0g350x`.
    pub family: String,

    /// URL for the datasheet.
    pub datasheet_url: String,

    /// URL for the reference manual.
    pub reference_manual_url: String,

    /// URL for the errata.
    pub errata_url: String,

    /// Memory layout.
    pub memory: Vec<Memory>,

    /// Packages which this chip is available in.
    pub packages: Vec<Package>,

    /// Mapping from device pin to IOMUX register index.
    pub iomux: BTreeMap<String, u32>,

    /// Which IO structure each device pin is built from, keyed the same way as `iomux`.
    ///
    /// Every pin with a PINCM has one.
    ///
    /// `mspm0-metapac-gen` flattens this onto each `Pin` as `Pin::structure`, so a consumer of the
    /// generated crate reads it there rather than here.
    pub io_structure: BTreeMap<String, IoStructure>,

    /// Device pins which have wakeup logic and can therefore wake the device from SHUTDOWN.
    ///
    /// The `FASTWAKE` mechanism, which wakes the device from STOP and STANDBY, works on any GPIO
    /// pin and is therefore not described here.
    ///
    /// `None` when sysconfig does not describe wakeup logic for this family, which is not the same as
    /// the family having no wake-capable pin. **That is the common case**: the attribute is absent
    /// on 11 of the 18 families, 30 of the 43 part numbers. The seven which carry it are c110x,
    /// c1105_c1106, g151x, g351x, g518x, l112x and l211x.
    ///
    /// Do not fall back to `io_structure` where this is `None`. SLAU846 Table 8-1 maps structure to
    /// wake capability and is wrong on mspm0c110x and msps003fx, whose open-drain pins have no
    /// wakeup logic — and msps003fx is one of the families with no attribute to contradict it.
    ///
    /// `mspm0-metapac-gen` flattens this onto each `Pin` as `Pin::wakeup`, so a consumer of the
    /// generated crate reads it there rather than here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wakeup_pins: Option<BTreeSet<String>>,

    /// The peripherals available on the chip.
    pub peripherals: BTreeMap<String, Peripheral>,

    /// Interrupts available on the chip.
    pub interrupts: BTreeMap<i32, Interrupt>,

    /// DMA channels available on the chip.
    pub dma_channels: BTreeMap<u32, DmaChannel>,

    /// Number configurable channels (MEMCTL) in the ADC peripheral.
    pub adc_memctl: u8,

    /// Number of options for VRSEL of the ADC peripheral.
    ///
    /// This is requried because we use a single adc_v1 pac for all chips.
    pub adc_vrsel: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// The name of the package.
    ///
    /// Example: `LQFP-64`
    pub name: String,

    /// The name of the chip this package applies to.
    ///
    /// This field exists as a result of the MSPS003 being MSPM0C110x with a different package.
    pub chip: String,

    /// The type of package.
    ///
    /// Example: `DGS28`
    pub package: String,

    /// The pins of the package.
    pub pins: Vec<PackagePin>,
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
///   datasheet's feature table has no wakeup column at all. Use [`Chip::wakeup_pins`].
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePin {
    /// The position by pin name.
    ///
    /// Examples:
    /// - `5`
    /// - `A4`
    pub position: String,

    /// The signals attached to this pin.
    ///
    /// Examples:
    /// - `PA0`
    /// - `NRST`
    pub signals: Vec<String>,
}

// TODO: The rest
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PeripheralType {
    /// Peripheral type is not known. This is an error if used when generating.
    #[default]
    Unknown,

    Adc,

    AesAdv,

    Aes,

    Canfd,

    Comp,

    Cpuss,

    Crc,

    Dac,

    Debugss,

    Dma,

    Event,

    /// This region contains read-only device constants such as the device id, flash and SRAM
    /// sizes and calibration values.
    FactoryRegion,

    FlashCtl,

    GpAmp,

    Gpio,

    I2c,

    I2s,

    Iomux,

    Iwdt,

    KeystoreCtl,

    Lcd,

    Lfss,

    Mathacl,

    Npu,

    Opa,

    Rtc,

    Spi,

    /// System Controller
    ///
    /// This peripheral may have a different version per part family.
    Sysctl,

    /// A timer.
    Tim,

    Trng,

    Uart,

    Unicomm,

    Usbfs,

    Vref,

    Wuc,

    Wwdt,
}

impl fmt::Display for PeripheralType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content = match self {
            PeripheralType::Unknown => "",
            PeripheralType::Adc => "adc",
            PeripheralType::Aes => "aes",
            PeripheralType::AesAdv => "aesadv",
            PeripheralType::Canfd => "canfd",
            PeripheralType::Comp => "comp",
            PeripheralType::Cpuss => "cpuss",
            PeripheralType::Crc => "crc",
            PeripheralType::Dac => "dac",
            PeripheralType::Debugss => "debugss",
            PeripheralType::Dma => "dma",
            PeripheralType::Event => "event",
            PeripheralType::FactoryRegion => "factoryregion",
            PeripheralType::FlashCtl => "flashctl",
            PeripheralType::GpAmp => "gpamp",
            PeripheralType::Gpio => "gpio",
            PeripheralType::I2c => "i2c",
            PeripheralType::I2s => "i2s",
            PeripheralType::Iomux => "iomux",
            PeripheralType::Iwdt => "iwdt",
            PeripheralType::KeystoreCtl => "keystorectl",
            PeripheralType::Lcd => "lcd",
            PeripheralType::Lfss => "lfss",
            PeripheralType::Mathacl => "mathacl",
            PeripheralType::Npu => "npu",
            PeripheralType::Opa => "opa",
            PeripheralType::Rtc => "rtc",
            PeripheralType::Spi => "spi",
            PeripheralType::Sysctl => "sysctl",
            PeripheralType::Tim => "tim",
            PeripheralType::Trng => "trng",
            PeripheralType::Uart => "uart",
            PeripheralType::Unicomm => "unicomm",
            PeripheralType::Usbfs => "usbfs",
            PeripheralType::Vref => "vref",
            PeripheralType::Wuc => "wuc",
            PeripheralType::Wwdt => "wwdt",
        };

        write!(f, "{content}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerDomain {
    /// "low speed" power domain. This power domain is powered in RUN, SLEEP, STOP and STANDBY modes.
    Pd0,

    /// "high performance" power domain. This power domain is powered in RUN and SLEEP modes.
    Pd1,

    /// PDB backup power domain. This is usually powered by VBAT.
    Backup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peripheral {
    pub name: String,

    #[serde(flatten, rename = "type")]
    pub ty: PeripheralType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<u32>,

    pub power_domain: PowerDomain,

    pub pins: Vec<PeripheralPin>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sys_fentries: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeripheralPin {
    /// The name of the pin that this peripheral can be bound to.
    ///
    /// e.g. `PA0`, `PC8`
    pub pin: String,

    /// The signal provided by the peripheral.
    ///
    /// e.g. `SCL`, `TX`
    pub signal: String,

    /// The pin function value for this pin that selects the signal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pf: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    pub name: String,
    pub num: i32,
    pub group: BTreeMap<u32, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmaChannel {
    /// Whether this is a full channel or basic channel.
    pub full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// The memory partition.
    pub name: String,

    /// Amount of memory in KB.
    pub length: u32,

    /// Address of the memory.
    pub address: u32,
}
