use super::Hotspot;
use windows::core::HSTRING;
use windows::Networking::Connectivity::NetworkInformation;
use windows::Networking::NetworkOperators::NetworkOperatorTetheringManager;
use windows::Networking::NetworkOperators::TetheringOperationalState;

pub(crate) struct HotspotWindows<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
}

impl<'a> Hotspot for HotspotWindows<'a> {
    async fn start(&self) -> color_eyre::eyre::Result<()> {
        let connection_profile = NetworkInformation::GetInternetConnectionProfile()?;
        let tethering_manager =
            NetworkOperatorTetheringManager::CreateFromConnectionProfile(&connection_profile)?;
        let configuration = tethering_manager.GetCurrentAccessPointConfiguration()?;

        if configuration.Ssid()? != self.ssid || configuration.Passphrase()? != self.password {
            configuration.SetSsid(&HSTRING::from(self.ssid))?;
            configuration.SetPassphrase(&HSTRING::from(self.password))?;
            tethering_manager
                .ConfigureAccessPointAsync(&configuration)?
                .await?;
        }

        if tethering_manager.TetheringOperationalState()? == TetheringOperationalState::Off {
            let result = tethering_manager.StartTetheringAsync()?.await?;
            if result.Status()?
                != windows::Networking::NetworkOperators::TetheringOperationStatus::Success
            {
                return Err(color_eyre::eyre::eyre!("Failed to start tethering"));
            }
        }
        Ok(())
    }

    async fn stop(&self) -> color_eyre::eyre::Result<()> {
        let connection_profile = NetworkInformation::GetInternetConnectionProfile()?;
        let tethering_manager =
            NetworkOperatorTetheringManager::CreateFromConnectionProfile(&connection_profile)?;
        if tethering_manager.TetheringOperationalState()?
            == windows::Networking::NetworkOperators::TetheringOperationalState::On
        {
            let result = tethering_manager.StopTetheringAsync()?.await?;
            if result.Status()?
                != windows::Networking::NetworkOperators::TetheringOperationStatus::Success
            {
                return Err(color_eyre::eyre::eyre!("Failed to stop tethering"));
            }
        }
        Ok(())
    }
}
