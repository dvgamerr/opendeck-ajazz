use std::collections::HashSet;
use hidapi::{DeviceInfo, HidApi, HidResult};
use crate::info::{is_mirabox_vendor, Kind};

const AKP05E_552A_USAGE_PAGE: u16 = 0xffa0;
const AKP05E_552A_USAGE: u16 = 0x0001;

pub(crate) fn is_supported_interface(device: &DeviceInfo, kind: Kind) -> bool {
    if kind != Kind::Akp05E552A {
        return true;
    }

    let usage = (device.usage_page(), device.usage());
    usage == (AKP05E_552A_USAGE_PAGE, AKP05E_552A_USAGE)
        || (usage == (0, 0) && device.interface_number() == 0)
}

/// Creates an instance of the HidApi
///
/// Can be used if you don't want to link hidapi crate into your project
pub fn new_hidapi() -> HidResult<HidApi> {
    HidApi::new()
}

/// Actually refreshes the device list
pub fn refresh_device_list(hidapi: &mut HidApi) -> HidResult<()> {
    hidapi.refresh_devices()
}

/// Returns a list of devices as (Kind, Serial Number) that could be found using HidApi.
///
/// **WARNING:** To refresh the list, use [refresh_device_list]
pub fn list_devices(hidapi: &HidApi) -> Vec<(Kind, String)> {
    hidapi
        .device_list()
        .filter_map(|d| {
            if !is_mirabox_vendor(d.vendor_id()) {
                return None;
            }

            let kind = Kind::from_vid_pid(d.vendor_id(), d.product_id())?;
            if !is_supported_interface(d, kind) {
                return None;
            }

            let serial = d.serial_number()?;
            Some((kind, serial.to_string()))
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}
