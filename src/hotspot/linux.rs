use super::Hotspot;
use color_eyre::eyre::Result;
use tokio::process::Command;
use regex::Regex;
use tracing::info;

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
        let ssid_regex = Regex::new(&format!(r"802-11-wireless\.ssid:\s*{}", self.ssid)).unwrap();
        let password_regex = Regex::new(&format!(r"802-11-wireless-security\.psk:\s*{}", self.password)).unwrap();
        let ssid_match = ssid_regex.is_match(&output_str);
        let password_match = password_regex.is_match(&output_str);
        
        if !ssid_match || !password_match {
            info!("Password or SSID don't match. Recreating AP.");
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
            .arg("wlP1p1s0")
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
    
    async fn show_password(&self) -> Result<()> {
        Command::new("nmcli")
            .arg("dev")
            .arg("wifi")
            .arg("show-password")
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
        self.ensure_hotspot_exists().await?;
        
        Command::new("nmcli")
            .arg("connection")
            .arg("up")
            .arg(self.name)
            .spawn()
            .unwrap()
            .wait()
            .await?;

        self.show_password().await?;
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
