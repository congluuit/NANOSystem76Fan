//! Driver recommendations for System76 Open Firmware machines running Windows.
//!
//! Source: <https://github.com/system76/windows-drivers> (README and
//! `drivers/intel/*/README.md`).

pub const REPO_URL: &str = "https://github.com/system76/windows-drivers";
pub const INPOUT_URL: &str = "https://www.highrez.co.uk/downloads/inpout32/";
pub const NVIDIA_URL: &str = "https://www.nvidia.com/Download/index.aspx";

/// Where the official driver list for a model lives (relative to the repo root,
/// or a README anchor for older models).
pub fn model_target(model: &str) -> Option<&'static str> {
    Some(match model {
        "addw3" | "addw4" | "bonw15" | "oryp12" | "serw13" => "drivers/intel/rpl-hx",
        "addw5" | "bonw16" | "serw14" => "drivers/intel/arl-hx",
        "addp6" | "lemp14" => "drivers/intel/ptl-h",
        "darp9" | "galp7" | "gaze18" | "lemp12" | "oryp11" => "drivers/intel/rpl-uph",
        "darp10" | "lemp13" => "drivers/intel/mtl-uh",
        "darp11" => "drivers/intel/arl-hu",
        "gaze17" | "lemp11" | "oryp9" | "oryp10" => "drivers/intel/adl-p",
        "addw2" => "#adder-ws-addw2",
        "bonw14" => "#bonobo-ws-bonw14",
        "darp6" => "#darter-pro-darp6",
        "darp7" => "#darter-pro-darp7",
        "darp8" => "#darter-pro-darp8",
        "galp4" => "#galago-pro-galp4",
        "galp5" => "#galago-pro-galp5",
        "gaze15" => "#gazelle-gaze15",
        "gaze16" => "#gazelle-gaze16",
        "lemp9" => "#lemur-pro-lemp9",
        "lemp10" => "#lemur-pro-lemp10",
        "oryp6" => "#oryx-pro-oryp6",
        "oryp7" => "#oryx-pro-oryp7",
        "oryp8" => "#oryx-pro-oryp8",
        "pang12" => "#pangolin-pang12",
        "pang13" => "#pangolin-pang13",
        "pang14" => "#pangolin-pang14",
        "pang15" => "#pangolin-pang15",
        _ => return None,
    })
}

pub fn model_url(model: &str) -> String {
    match model_target(model) {
        Some(t) if t.starts_with('#') => format!("{REPO_URL}{t}"),
        Some(t) => format!("{REPO_URL}/tree/master/{t}"),
        None => REPO_URL.to_string(),
    }
}

/// A driver package from the platform folder, as listed by System76.
pub struct Package {
    pub name: &'static str,
    pub version: &'static str,
    pub installer: &'static str,
}

/// Packages of the `rpl-hx` folder (2024-WW28), used by Adder WS addw3/addw4,
/// Bonobo WS bonw15, Oryx Pro oryp12 and Serval WS serw13.
pub const RPL_HX: &[Package] = &[
    Package { name: "Intel Chipset", version: "10.1.19899.8597", installer: "SetupChipset.exe" },
    Package { name: "Intel Serial IO", version: "30.100.2417.30", installer: "SetupSerialIO.exe" },
    Package { name: "Intel Thunderbolt", version: "Rev 93", installer: "Thunderbolt(TM) Software Installer" },
    Package { name: "Intel Wi-Fi", version: "23.60.1.2G", installer: "UWD/Win64/Installer/WirelessSetup.exe" },
];

pub fn packages_for(model: &str) -> &'static [Package] {
    match model_target(model) {
        Some("drivers/intel/rpl-hx") => RPL_HX,
        _ => &[],
    }
}

/// Well-known devices that show up without a driver on System76 Open Firmware.
pub enum KnownDevice {
    /// `ACPI\17761776`: the System76 ACPI device (keyboard backlight, hotkeys,
    /// battery thresholds on Linux). There is no Windows driver; the EC still
    /// handles the hotkeys itself.
    System76Acpi,
    /// `ACPI\BOOT0000`: coreboot table device, only used by Linux.
    Coreboot,
    Unknown,
}

pub fn classify(device_id: &str) -> KnownDevice {
    let id = device_id.to_ascii_uppercase();
    if id.starts_with("ACPI\\17761776") {
        KnownDevice::System76Acpi
    } else if id.starts_with("ACPI\\BOOT0000") {
        KnownDevice::Coreboot
    } else {
        KnownDevice::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(model_url("addw3"), "https://github.com/system76/windows-drivers/tree/master/drivers/intel/rpl-hx");
        assert_eq!(model_url("addw2"), "https://github.com/system76/windows-drivers#adder-ws-addw2");
        assert_eq!(model_url("nope"), REPO_URL);
        assert_eq!(packages_for("addw3").len(), 4);
    }

    #[test]
    fn classify_devices() {
        assert!(matches!(classify("ACPI\\17761776\\0"), KnownDevice::System76Acpi));
        assert!(matches!(classify("acpi\\boot0000\\0"), KnownDevice::Coreboot));
        assert!(matches!(classify("PCI\\VEN_8086"), KnownDevice::Unknown));
    }
}
