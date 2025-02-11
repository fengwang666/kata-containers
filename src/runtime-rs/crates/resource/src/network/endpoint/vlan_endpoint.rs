// Copyright (c) 2019-2022 Alibaba Cloud
// Copyright (c) 2019-2022 Ant Group
//
// SPDX-License-Identifier: Apache-2.0
//

use std::io::{self, Error};

use anyhow::{Context, Result};
use async_trait::async_trait;
use hypervisor::device::DeviceType;
use hypervisor::NetworkDevice;

use super::endpoint_persist::{EndpointState, VlanEndpointState};
use super::Endpoint;
use crate::network::network_model::TC_FILTER_NET_MODEL_STR;
use crate::network::{utils, NetworkPair};
use hypervisor::{device::driver::NetworkConfig, Hypervisor};
#[derive(Debug)]
pub struct VlanEndpoint {
    pub(crate) net_pair: NetworkPair,
}

impl VlanEndpoint {
    pub async fn new(
        handle: &rtnetlink::Handle,
        name: &str,
        idx: u32,
        queues: usize,
    ) -> Result<Self> {
        let net_pair = NetworkPair::new(handle, idx, name, TC_FILTER_NET_MODEL_STR, queues)
            .await
            .context("error creating networkInterfacePair")?;
        Ok(VlanEndpoint { net_pair })
    }

    fn get_network_config(&self) -> Result<NetworkConfig> {
        let iface = &self.net_pair.tap.tap_iface;
        let guest_mac = utils::parse_mac(&iface.hard_addr).ok_or_else(|| {
            Error::new(
                io::ErrorKind::InvalidData,
                format!("hard_addr {}", &iface.hard_addr),
            )
        })?;
        Ok(NetworkConfig {
            host_dev_name: iface.name.clone(),
            guest_mac: Some(guest_mac),
        })
    }
}

#[async_trait]
impl Endpoint for VlanEndpoint {
    async fn name(&self) -> String {
        self.net_pair.virt_iface.name.clone()
    }

    async fn hardware_addr(&self) -> String {
        self.net_pair.tap.tap_iface.hard_addr.clone()
    }

    async fn attach(&self, h: &dyn Hypervisor) -> Result<()> {
        self.net_pair
            .add_network_model()
            .await
            .context("error adding network model")?;
        let config = self.get_network_config().context("get network config")?;
        h.add_device(DeviceType::Network(NetworkDevice {
            id: self.net_pair.virt_iface.name.clone(),
            config,
        }))
        .await
        .context("error adding device by hypervisor")?;

        Ok(())
    }

    async fn detach(&self, h: &dyn Hypervisor) -> Result<()> {
        self.net_pair
            .del_network_model()
            .await
            .context("error deleting network model")?;
        let config = self
            .get_network_config()
            .context("error getting network config")?;
        h.remove_device(DeviceType::Network(NetworkDevice {
            id: self.net_pair.virt_iface.name.clone(),
            config,
        }))
        .await
        .context("error removing device by hypervisor")?;

        Ok(())
    }

    async fn save(&self) -> Option<EndpointState> {
        Some(EndpointState {
            vlan_endpoint: Some(VlanEndpointState {
                if_name: self.net_pair.virt_iface.name.clone(),
                network_qos: self.net_pair.network_qos,
            }),
            ..Default::default()
        })
    }
}
