use crate::{error::RegionError, Error, Result};
use helium_proto::{
    BlockchainRegionParamV1, GatewayRegionParamsStreamedRespV1, Region as ProtoRegion,
    RegionSpreading, TaggedSpreading,
};
use rust_decimal::Decimal;
use serde::{de, Deserialize, Deserializer};
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Region(ProtoRegion);

impl From<Region> for ProtoRegion {
    fn from(v: Region) -> Self {
        v.0
    }
}

impl AsRef<ProtoRegion> for Region {
    fn as_ref(&self) -> &ProtoRegion {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Region {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RegionVisitor;

        impl<'de> de::Visitor<'de> for RegionVisitor {
            type Value = Region;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("region string")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Region, E>
            where
                E: de::Error,
            {
                let proto_region = match value {
                    "US915" => ProtoRegion::Us915,
                    "EU868" => ProtoRegion::Eu868,
                    "EU433" => ProtoRegion::Eu433,
                    "CN470" => ProtoRegion::Cn470,
                    "CN779" => ProtoRegion::Cn779,
                    "AU915" => ProtoRegion::Au915,
                    "AS923_1" => ProtoRegion::As9231,
                    "AS923_1B" => ProtoRegion::As9231b,
                    "AS923_2" => ProtoRegion::As9232,
                    "AS923_3" => ProtoRegion::As9233,
                    "AS923_4" => ProtoRegion::As9234,
                    "KR920" => ProtoRegion::Kr920,
                    "IN865" => ProtoRegion::In865,
                    "CD900_1A" => ProtoRegion::Cd9001a,
                    unsupported => {
                        return Err(de::Error::custom(format!(
                            "unsupported region: {unsupported}"
                        )))
                    }
                };
                Ok(Region(proto_region))
            }
        }

        deserializer.deserialize_str(RegionVisitor)
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ProtoRegion::Us915 => f.write_str("US915"),
            ProtoRegion::Eu868 => f.write_str("EU868"),
            ProtoRegion::Eu433 => f.write_str("EU433"),
            ProtoRegion::Cn470 => f.write_str("CN470"),
            ProtoRegion::Cn779 => f.write_str("CN779"),
            ProtoRegion::Au915 => f.write_str("AU915"),
            ProtoRegion::As9231 => f.write_str("AS923_1"),
            ProtoRegion::As9231b => f.write_str("AS923_1B"),
            ProtoRegion::As9232 => f.write_str("AS923_2"),
            ProtoRegion::As9233 => f.write_str("AS923_3"),
            ProtoRegion::As9234 => f.write_str("AS923_4"),
            ProtoRegion::Kr920 => f.write_str("KR920"),
            ProtoRegion::In865 => f.write_str("IN865"),
            ProtoRegion::Cd9001a => f.write_str("CD900_1A"),
        }
    }
}

impl From<Region> for i32 {
    fn from(region: Region) -> Self {
        region.0.into()
    }
}

impl From<&Region> for i32 {
    fn from(region: &Region) -> Self {
        region.0.into()
    }
}

impl Region {
    pub fn from_i32(v: i32) -> Result<Self> {
        ProtoRegion::from_i32(v)
            .map(Self)
            .ok_or_else(|| Error::custom(format!("unsupported region {v}")))
    }
}

impl slog::Value for Region {
    fn serialize(
        &self,
        _record: &slog::Record,
        key: slog::Key,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        serializer.emit_str(key, &self.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RegionParams {
    pub gain: Decimal,
    pub region: Region,
    pub params: Vec<BlockchainRegionParamV1>,
}

impl TryFrom<GatewayRegionParamsStreamedRespV1> for RegionParams {
    type Error = Error;
    fn try_from(value: GatewayRegionParamsStreamedRespV1) -> Result<Self> {
        let region = Region::from_i32(value.region)?;
        let params = if let Some(params) = value.params {
            params.region_params
        } else {
            return Err(RegionError::no_region_params());
        };
        Ok(Self {
            gain: Decimal::new(value.gain as i64, 1),
            params,
            region,
        })
    }
}

impl RegionParams {
    pub fn max_eirp(&self) -> Option<Decimal> {
        self.params
            .iter()
            .max_by_key(|p| p.max_eirp)
            .map(|v| Decimal::new(v.max_eirp as i64, 1))
    }

    pub fn tx_power(&self) -> Option<u32> {
        use rust_decimal::prelude::ToPrimitive;
        self.max_eirp()
            .and_then(|max_eirp| (max_eirp - self.gain).trunc().to_u32())
    }

    pub fn to_string(v: &Option<Self>) -> String {
        match v {
            None => "none".to_string(),
            Some(params) => params.region.to_string(),
        }
    }

    pub fn spreading(&self, packet_size: u32) -> Option<&'static str> {
        // The spreading does not change per channel frequency, so just get one
        // and do selection depending on max_packet_size
        self.params
            .first()
            .and_then(|param| param.spreading.as_ref())
            .map(|spreading| &spreading.tagged_spreading)
            .and_then(|tagged_spreading| {
                tagged_spreading
                    .iter()
                    .find(|ts| ts.max_packet_size >= packet_size)
            })
            .and_then(spreading_to_str)
    }

    pub fn bandwidth(&self) -> Option<u32> {
        // The bandwidth does not change per channel frequency, so just get one
        self.params.first().map(|p| p.bandwidth)
    }

    pub fn datarate(&self, packet_size: u32) -> Option<String> {
        self.spreading(packet_size).and_then(|spreading| {
            self.bandwidth()
                .map(|bw| (bw / 1000) as u32)
                .map(|bk| format!("{spreading}BW{bk}"))
        })
    }

    pub fn channel(&self, frequency: f32) -> Option<i32> {
        let mut channel: i32 = 0;
        for param in &self.params {
            if (param.channel_frequency as f64 - frequency as f64).abs() <= 0.001 {
                return Some(channel);
            } else {
                channel += 1;
            }
        }
        None
    }
}

fn spreading_to_str(spreading: &TaggedSpreading) -> Option<&'static str> {
    RegionSpreading::from_i32(spreading.region_spreading).and_then(|rs| match rs {
        RegionSpreading::Sf7 => Some("SF7"),
        RegionSpreading::Sf8 => Some("SF8"),
        RegionSpreading::Sf9 => Some("SF9"),
        RegionSpreading::Sf10 => Some("SF10"),
        RegionSpreading::Sf11 => Some("SF11"),
        RegionSpreading::Sf12 => Some("SF12"),
        RegionSpreading::SfInvalid => None,
    })
}
