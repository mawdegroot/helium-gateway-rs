use crate::{Error, RegionParams, Result};
use helium_proto::{
    gateway_resp_v1, GatewayConfigUpdateStreamedRespV1, GatewayPocChallengeNotificationRespV1,
    GatewayRespV1, GatewayScFollowStreamedRespV1, Routing,
};

pub(crate) trait Response {
    fn height(&self) -> u64;
    fn routings(&self) -> Result<&[Routing]>;
    fn region_params(&self) -> Result<RegionParams>;
    fn state_channel_response(&self) -> Result<&GatewayScFollowStreamedRespV1>;
    fn poc_challenge(&self) -> Result<&GatewayPocChallengeNotificationRespV1>;
    fn config_update(&self) -> Result<&GatewayConfigUpdateStreamedRespV1>;
}

macro_rules! match_response {
    ($res:expr, $pattern:pat_param, $expression:expr) => {
        match &$res.msg {
            Some($pattern) => $expression,
            msg => Err(Error::custom(
                format!("Unexpected gateway message {msg:?}",),
            )),
        }
    };
}

impl Response for GatewayRespV1 {
    fn height(&self) -> u64 {
        self.height
    }

    fn routings(&self) -> Result<&[Routing]> {
        match_response!(
            self,
            gateway_resp_v1::Msg::RoutingStreamedResp(routings),
            Ok(&routings.routings)
        )
    }

    fn region_params(&self) -> Result<RegionParams> {
        match &self.msg {
            Some(gateway_resp_v1::Msg::RegionParamsStreamedResp(params)) => {
                RegionParams::try_from(params.to_owned())
            }
            msg => Err(Error::custom(
                format!("Unexpected gateway message {msg:?}",),
            )),
        }
    }

    fn state_channel_response(&self) -> Result<&GatewayScFollowStreamedRespV1> {
        match_response!(self, gateway_resp_v1::Msg::FollowStreamedResp(res), Ok(res))
    }

    fn poc_challenge(&self) -> Result<&GatewayPocChallengeNotificationRespV1> {
        match_response!(self, gateway_resp_v1::Msg::PocChallengeResp(res), Ok(res))
    }

    fn config_update(&self) -> Result<&GatewayConfigUpdateStreamedRespV1> {
        match_response!(
            self,
            gateway_resp_v1::Msg::ConfigUpdateStreamedResp(res),
            Ok(res)
        )
    }
}
