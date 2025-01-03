use super::api::Hotspot;
use color_eyre::eyre::Result;
use tokio::process::Command;


pub(crate) struct HotspotLinux<'a> {
    pub name: &'a str,
    pub ssid: &'a str,
    pub password: &'a str,
}

impl<'a> HotspotLinux<'a> {
    async fn ensure_hotspot_exists(&self) -> Result<()> {
        let output = Command::new("nmcli")
            .arg("connection")
            .arg("show")
            .arg(self.name)
            .output()
            .await?;
        if !output.status.success() {
            self.create_hotspot().await?;
            return Ok(());
        }
        let output_str = String::from_utf8_lossy(&output.stdout);
        let ssid_match = output_str.contains(&format!("802-11-wireless.ssid:{}", self.ssid));
        let password_match =
            output_str.contains(&format!("802-11-wireless-security.psk:{}", self.password));
        if !ssid_match || !password_match {
            self.delete_hotspot().await?;
            return self.create_hotspot().await;
        }

        Ok(())
    }

    async fn create_hotspot(&self) -> Result<()> {
        Command::new("nmcli")
            .arg("device")
            .arg("wifi")
            .arg("hotspot")
            .arg("ifname")
            .arg("wlan0")
            .arg("con-name")
            .arg(self.name)
            .arg("ssid")
            .arg(self.ssid)
            .arg("password")
            .arg(self.password)
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }

    async fn delete_hotspot(&self) -> Result<()> {
        Command::new("nmcli")
            .arg("connection")
            .arg("delete")
            .arg(self.name)
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }
}

impl<'a> Hotspot for HotspotLinux<'a> {
    async fn start(&self) -> color_eyre::eyre::Result<()> {
        Command::new("nmcli")
            .arg("connection")
            .arg("up")
            .arg(self.name)
            .spawn()
            .unwrap()
            .wait()
            .await?;

        Ok(())
    }

    async fn stop(&self) -> color_eyre::eyre::Result<()> {
        Command::new("nmcli")
            .arg("connection")
            .arg("down")
            .arg(self.name)
            .spawn()
            .unwrap()
            .wait()
            .await?;

        Ok(())
    }
}
