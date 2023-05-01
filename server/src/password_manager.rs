use std::{collections::HashMap, path};

use tokio::{
    fs::{create_dir_all, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

pub struct PasswordManager {
    passwords: Mutex<HashMap<String, String>>,
    file: Mutex<File>,
}

impl PasswordManager {
    pub async fn new(filename: String) -> anyhow::Result<Self> {
        create_dir_all(path::Path::new(&filename).parent().unwrap()).await?;
        let passwords = Mutex::new(HashMap::new());
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .await?;
        let mut lines = String::new();
        file.read_to_string(&mut lines).await?;
        for line in lines.lines() {
            let mut parts = line.split(' ');
            let login = parts.next().unwrap();
            let _ip = parts.next().unwrap();
            let password = parts.next().unwrap();
            passwords
                .lock()
                .await
                .insert(login.to_string(), password.to_string());
        }
        Ok(PasswordManager {
            passwords,
            file: Mutex::new(file),
        })
    }

    pub async fn check_password(
        &self,
        login: &str,
        password: &str,
        ip: &str,
    ) -> anyhow::Result<()> {
        log::info!("Checking password for {} from {}", login, ip);
        if password == "GO" {
            anyhow::bail!("Please don't use 'GO' as your password!");
        }
        let expected_password = self.passwords.lock().await.get(login).cloned();
        if let Some(expected_password) = expected_password {
            log::info!("Existing password...");
            if expected_password == password {
                return Ok(());
            } else {
                return Err(anyhow::anyhow!(
                    "Wrong password. Use the same password as before."
                ));
            }
        } else {
            log::info!("Inserting new password");
            self.passwords
                .lock()
                .await
                .insert(login.to_string(), password.to_string());
            log::info!("Done, saving to file...");
            let mut guard = self.file.lock().await;
            guard
                .write_all(format!("{login} {ip} {password}\n").as_bytes())
                .await?;
            guard.flush().await?;
            log::info!("Updated passwords file");
            Ok(())
        }
    }
}
