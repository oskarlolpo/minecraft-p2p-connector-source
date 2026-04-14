use anyhow::{Context, Result};
use rupnp::ssdp::{SearchTarget, URN};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;
use tracing::{error, info, warn};

/// Хранит состояние проброшенного порта. 
/// При выходе из области видимости (Drop) порт будет автоматически закрыт на роутере.
pub struct UpnpMapping {
    device: rupnp::Device,
    service_urn: URN,
    external_port: u16,
}

const WAN_IP_CONNECTION: URN = URN::service("schemas-upnp-org", "WANIPConnection", 1);
const WAN_PPP_CONNECTION: URN = URN::service("schemas-upnp-org", "WANPPPConnection", 1);

impl UpnpMapping {
    /// Пытается найти UPnP-шлюз и пробросить указанный UDP-порт.
    pub async fn attempt_map(local_port: u16, description: &str) -> Result<Self> {
        let local_ip = Self::get_local_ip()
            .context("Не удалось определить локальный IPv4 адрес хоста")?;
        
        info!("UPnP: Начинаем поиск устройств (5s) через IP {}...", local_ip);
        
        let search_target = SearchTarget::RootDevice;
        let devices = rupnp::discover(&search_target, Duration::from_secs(5), None)
            .await
            .context("Ошибка при запуске UPnP дискавери")?;
        
        let mut devices = std::pin::pin!(devices);
        use futures_util::StreamExt;

        while let Some(device) = devices.next().await {
            let device = match device {
                Ok(d) => d,
                Err(_) => continue,
            };

            // Ищем подходящий сервис (IP или PPP соединение)
            let service_urn = if device.find_service(&WAN_IP_CONNECTION).is_some() {
                Some(WAN_IP_CONNECTION)
            } else if device.find_service(&WAN_PPP_CONNECTION).is_some() {
                Some(WAN_PPP_CONNECTION)
            } else {
                None
            };

            if let Some(urn) = service_urn {
                let service = device.find_service(&urn).unwrap();
                info!("UPnP: Найден шлюз '{}', сервис {}", device.friendly_name(), urn);

                let args = format!(
                    "<NewRemoteHost></NewRemoteHost>\
                     <NewExternalPort>{}</NewExternalPort>\
                     <NewProtocol>UDP</NewProtocol>\
                     <NewInternalPort>{}</NewInternalPort>\
                     <NewInternalClient>{}</NewInternalClient>\
                     <NewEnabled>1</NewEnabled>\
                     <NewPortMappingDescription>{}</NewPortMappingDescription>\
                     <NewLeaseDuration>0</NewLeaseDuration>",
                    local_port, local_port, local_ip, description
                );

                match service.action(device.url(), "AddPortMapping", &args).await {
                    Ok(_) => {
                        info!("UPnP: Порт {} (UDP) успешно проброшен на шлюзе {}", local_port, device.friendly_name());
                        return Ok(Self {
                            device,
                            service_urn: urn,
                            external_port: local_port,
                        });
                    }
                    Err(e) => {
                        warn!("UPnP: Ошибка AddPortMapping на '{}': {}. Ищем дальше...", device.friendly_name(), e);
                        continue;
                    }
                }
            }
        }

        anyhow::bail!("UPnP-шлюз не найден или не поддерживает проброс портов (проверены все доступные)")
    }

    fn get_local_ip() -> Result<Ipv4Addr> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect("8.8.8.8:80")?;
        let local_addr = socket.local_addr()?;
        match local_addr.ip() {
            std::net::IpAddr::V4(ipv4) => Ok(ipv4),
            _ => anyhow::bail!("IPv4 не найден"),
        }
    }
}

impl Drop for UpnpMapping {
    fn drop(&mut self) {
        let device = self.device.clone();
        let urn = self.service_urn.clone();
        let port = self.external_port;

        info!("UPnP: Запущена очистка порта {} на '{}'...", port, device.friendly_name());

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            
            rt.block_on(async {
                if let Some(service) = device.find_service(&urn) {
                    let args = format!(
                        "<NewRemoteHost></NewRemoteHost>\
                         <NewExternalPort>{}</NewExternalPort>\
                         <NewProtocol>UDP</NewProtocol>",
                        port
                    );
                    
                    match service.action(device.url(), "DeletePortMapping", &args).await {
                        Ok(_) => info!("UPnP: Порт {} успешно удален.", port),
                        Err(e) => error!("UPnP: Ошибка удаления порта {}: {}", port, e),
                    }
                }
            });
        });
    }
}
