use crate::{
    protocol::codes,
    images::{ImageFormat, ImageMirroring, ImageMode, ImageRotation},
    AjazzError,
};

/// Returns the display name for a supported device kind.
#[cfg(feature = "async")]
pub fn get_product_name(kind: &Kind) -> String {
    match kind {
        Kind::Akp153 => "Ajazz AKP153",
        Kind::Akp153E => "Ajazz AKP153E",
        Kind::Akp153R => "Ajazz AKP153R",
        Kind::Akp815 => "Ajazz AKP815",
        Kind::Akp03 => "Ajazz AKP03",
        Kind::Akp03E => "Ajazz AKP03E",
        Kind::Akp03R => "Ajazz AKP03R",
        Kind::Akp03RRev2 => "Ajazz AKP03R rev 2",
        Kind::Akp05E552A => "Ajazz AKP05E_552A",
    }
    .to_string()
}

/// Returns true for vendors IDs that are handled by the library
pub const fn is_mirabox_vendor(vendor: u16) -> bool {
    matches!(
        vendor,
        codes::VENDOR_ID_MIRABOX_V1 | codes::VENDOR_ID_MIRABOX_V2
    )
}

/// Enum describing kinds of Ajazz devices
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Kind {
    /// Ajazz AKP153
    Akp153,
    /// Ajazz AKP153E
    Akp153E,
    /// Ajazz AKP153R
    Akp153R,
    /// Ajazz AKP815
    Akp815,
    /// Ajazz AKP03
    Akp03,
    /// Ajazz AKP03E
    Akp03E,
    /// Ajazz AKP03R
    Akp03R,
    /// Ajazz AKP03R rev 2
    Akp03RRev2,
    /// Ajazz AKP05E_552A
    Akp05E552A,
}

impl Kind {
    /// Creates [Kind] variant from Vendor ID and Product ID
    pub const fn from_vid_pid(vid: u16, pid: u16) -> Option<Kind> {
        match vid {
            codes::VENDOR_ID_MIRABOX_V1 => match pid {
                codes::PID_AJAZZ_AKP153 => Some(Kind::Akp153),
                codes::PID_AJAZZ_AKP815 => Some(Kind::Akp815),
                _ => None,
            },

            codes::VENDOR_ID_MIRABOX_V2 => match pid {
                codes::PID_AJAZZ_AKP153E => Some(Kind::Akp153E),
                codes::PID_AJAZZ_AKP153R => Some(Kind::Akp153R),
                codes::PID_AJAZZ_AKP03 => Some(Kind::Akp03),
                codes::PID_AJAZZ_AKP03E => Some(Kind::Akp03E),
                codes::PID_AJAZZ_AKP03R => Some(Kind::Akp03R),
                codes::PID_AJAZZ_AKP03R_REV2 => Some(Kind::Akp03RRev2),
                codes::PID_AJAZZ_AKP05E_552A => Some(Kind::Akp05E552A),
                _ => None,
            },

            _ => None,
        }
    }

    /// Amount of touch points the Deck kind has
    pub fn touchpoint_count(&self) -> u8 {
        match self {
            Kind::Akp05E552A => 4,
            _ => 0,
        }
    }

    /// Maps an OpenDeck key index to the device's native key index.
    pub fn opendeck_to_device_key(&self, key: u8) -> Result<u8, AjazzError> {
        if key == 0xff {
            return Ok(key);
        }
        if key >= self.display_key_count() {
            return Err(AjazzError::InvalidKeyIndex(key));
        }

        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => {
                if key < self.key_count() {
                    Ok(
                        [0, 3, 6, 9, 12, 15, 1, 4, 7, 10, 13, 16, 2, 5, 8, 11, 14, 17]
                            [key as usize],
                    )
                } else {
                    Ok(key)
                }
            }

            Kind::Akp815 => {
                if key < self.key_count() {
                    Ok([0, 3, 6, 9, 12, 1, 4, 7, 10, 13, 2, 5, 8, 11, 14][key as usize])
                } else {
                    Ok(key)
                }
            }

            Kind::Akp05E552A => {
                if key < self.key_count() {
                    Ok([10, 11, 12, 13, 14, 5, 6, 7, 8, 9][key as usize])
                } else {
                    Ok(key)
                }
            }

            _ => Ok(key),
        }
    }

    /// Retrieves Product ID of the device
    pub const fn product_id(&self) -> u16 {
        match self {
            Kind::Akp153 => codes::PID_AJAZZ_AKP153,
            Kind::Akp153E => codes::PID_AJAZZ_AKP153E,
            Kind::Akp153R => codes::PID_AJAZZ_AKP153R,
            Kind::Akp815 => codes::PID_AJAZZ_AKP815,
            Kind::Akp03 => codes::PID_AJAZZ_AKP03,
            Kind::Akp03E => codes::PID_AJAZZ_AKP03E,
            Kind::Akp03R => codes::PID_AJAZZ_AKP03R,
            Kind::Akp03RRev2 => codes::PID_AJAZZ_AKP03R_REV2,
            Kind::Akp05E552A => codes::PID_AJAZZ_AKP05E_552A,
        }
    }

    /// Retrieves Vendor ID
    pub const fn vendor_id(&self) -> u16 {
        match self {
            Kind::Akp153 => codes::VENDOR_ID_MIRABOX_V1,
            Kind::Akp153E => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp153R => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp815 => codes::VENDOR_ID_MIRABOX_V1,
            Kind::Akp03 => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp03E => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp03R => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp03RRev2 => codes::VENDOR_ID_MIRABOX_V2,
            Kind::Akp05E552A => codes::VENDOR_ID_MIRABOX_V2,
        }
    }

    /// Amount of keys the device has
    pub const fn key_count(&self) -> u8 {
        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => 15 + 3,
            Kind::Akp815 => 15,
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => 6 + 3,
            Kind::Akp05E552A => 10,
        }
    }

    /// Amount of display keys the device has
    pub const fn display_key_count(&self) -> u8 {
        match self {
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => 6,
            Kind::Akp05E552A => 10,
            _ => self.key_count(),
        }
    }

    /// Amount of button rows the device has
    pub const fn row_count(&self) -> u8 {
        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => 3,
            Kind::Akp815 => 5,
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => 2,
            Kind::Akp05E552A => 2,
        }
    }

    /// Amount of button columns the device has
    pub const fn column_count(&self) -> u8 {
        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => 6,
            Kind::Akp815 => 3,
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => 3,
            Kind::Akp05E552A => 5,
        }
    }

    /// Amount of encoders/knobs the device has
    pub const fn encoder_count(&self) -> u8 {
        match self {
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => 3,
            Kind::Akp05E552A => 4,
            _ => 0,
        }
    }

    /// Size of the LCD strip on the device
    pub const fn lcd_strip_size(&self) -> Option<(usize, usize)> {
        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => Some((854, 480)),
            Kind::Akp815 => Some((800, 480)),
            Kind::Akp05E552A => Some((800, 100)),
            _ => None,
        }
    }

    /// Image format used by LCD screen, used for filling LCD
    pub fn lcd_image_format(&self) -> Option<ImageFormat> {
        match self {
            Kind::Akp05E552A => Some(ImageFormat {
                mode: ImageMode::JPEG,
                size: (800, 100),
                rotation: ImageRotation::Rot180,
                mirror: ImageMirroring::None,
            }),
            _ => None,
        }
    }

    /// Image format used by each action zone on the AKP05E_552A touchscreen strip.
    pub fn touch_zone_image_format(&self) -> Option<ImageFormat> {
        match self {
            Kind::Akp05E552A => Some(ImageFormat {
                mode: ImageMode::JPEG,
                size: (176, 112),
                rotation: ImageRotation::Rot180,
                mirror: ImageMirroring::None,
            }),
            _ => None,
        }
    }

    /// Size of the boot logo on the device
    pub const fn boot_logo_size(&self) -> Option<(usize, usize)> {
        match self {
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => Some((320, 240)),
            _ => self.lcd_strip_size(),
        }
    }

    /// Key layout of the device kind as (rows, columns)
    pub const fn key_layout(&self) -> (u8, u8) {
        (self.row_count(), self.column_count())
    }

    /// Image format used by the device kind
    pub const fn logo_image_format(&self) -> ImageFormat {
        match self {
            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R | Kind::Akp03RRev2 => ImageFormat {
                mode: ImageMode::JPEG,
                size: (240, 320),
                rotation: ImageRotation::Rot90,
                mirror: ImageMirroring::None,
            },

            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => ImageFormat {
                mode: ImageMode::JPEG,
                size: (854, 480),
                rotation: ImageRotation::Rot0,
                mirror: ImageMirroring::None,
            },

            Kind::Akp815 | Kind::Akp05E552A => ImageFormat {
                mode: ImageMode::JPEG,
                size: (800, 480),
                rotation: ImageRotation::Rot0,
                mirror: ImageMirroring::None,
            },
        }
    }

    /// Image format used by the device kind
    pub const fn key_image_format(&self) -> ImageFormat {
        match self {
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => ImageFormat {
                mode: ImageMode::JPEG,
                size: (85, 85),
                rotation: ImageRotation::Rot90,
                mirror: ImageMirroring::Both,
            },

            Kind::Akp815 | Kind::Akp05E552A => ImageFormat {
                mode: ImageMode::JPEG,
                size: (100, 100),
                rotation: ImageRotation::Rot180,
                mirror: ImageMirroring::None,
            },

            Kind::Akp03 | Kind::Akp03E | Kind::Akp03R => ImageFormat {
                mode: ImageMode::JPEG,
                size: (60, 60),
                rotation: ImageRotation::Rot0,
                mirror: ImageMirroring::None,
            },

            Kind::Akp03RRev2 => ImageFormat {
                mode: ImageMode::JPEG,
                size: (64, 64),
                rotation: ImageRotation::Rot90,
                mirror: ImageMirroring::None,
            },
        }
    }

    /// Returns true for devices with 512 byte packet length
    pub const fn is_v1_api(&self) -> bool {
        matches!(
            self,
            Kind::Akp153 | Kind::Akp153E | Kind::Akp153R | Kind::Akp815
        )
    }

    /// Returns true for devices with 1024 byte packet length
    pub const fn is_v2_api(&self) -> bool {
        matches!(self, Kind::Akp03 | Kind::Akp05E552A)
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;

    #[test]
    fn clear_all_sentinel_is_not_rejected_as_a_key_index() {
        assert!(matches!(
            Kind::Akp05E552A.opendeck_to_device_key(0xff),
            Ok(0xff)
        ));
    }
}
