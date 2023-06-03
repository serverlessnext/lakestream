mod crypto;
mod error;
mod secure_storage;
mod storage;
mod string_ops;

use std::collections::HashMap;

use crypto::{derive_crypto_key, derive_key_from_password, hash_username};
pub use error::SecureStringError;
use secure_storage::SecureStorage;
use serde_json;
use string_ops::generate_password;
use web_sys::CryptoKey;

pub type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Clone, PartialEq, Debug)]
pub struct ObjectKey {
    pub tag: String,
    pub id: String,
}

impl ObjectKey {
    pub fn new_with_form_tag(id: String) -> Self {
        Self {
            tag: "FORM".to_string(),
            id,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct StringVault {
    secure_storage: SecureStorage,
    username: String,
    hashed_username: String,
}

impl StringVault {
    pub async fn new(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let hashed_username = hash_username(username);
        let crypto_key =
            derive_key_from_password(&hashed_username, password).await?;
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };
        let secure_storage = SecureStorage::new(object_key, crypto_key);
        Ok(Self {
            secure_storage,
            username: username.to_string(),
            hashed_username,
        })
    }

    pub async fn user_exists(username: &str) -> bool {
        let hashed_username = hash_username(username);
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };
        SecureStorage::exists(object_key).await
    }

     pub async fn new_and_validate(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        let vault = StringVault::new(username, password).await?;

        // Try to load the passwords to validate the password
        match vault.secure_storage.load().await {
            Ok(_) => Ok(vault),
            Err(err) => match err {
                SecureStringError::NoLocalStorageData => {
                    // user is not yet created
                    Ok(vault)
                }
                SecureStringError::DecryptError(_) => {
                    // user exists but password is wrong
                    // TODO: offer reset password option
                    Err(err)
                }
                _ => Err(err), // Propagate any other errors
            },
        }
    }

    pub async fn validate_password(username: &str, password: &str) -> Result<bool, SecureStringError> {
        match Self::new_and_validate(username, password).await {
            Ok(_) => Ok(true), // Password is valid if new_and_validate doesn't return an error
            Err(SecureStringError::DecryptError(_)) => Ok(false), // DecryptError indicates an invalid password
            Err(err) => Err(err), // Propagate any other errors
        }
    }

    pub async fn change_password(username: &str, old_password: &str, new_password: &str) -> SecureStringResult<()> {
        // TODO: implemement
        Ok(())
    }

    pub async fn new_and_create(
        username: &str,
        password: &str,
    ) -> SecureStringResult<Self> {
        // TODO: check if user exists
        StringVault::reset_vault(username).await?;
        let vault = StringVault::new(username, password).await?;

        // Try to load the passwords to validate the password
        match vault.secure_storage.load().await {
            Ok(_) => Ok(vault),
            Err(err) => match err {
                SecureStringError::NoLocalStorageData => {
                    // user is not yet created
                    Ok(vault)
                }
                SecureStringError::DecryptError(_) => {
                    // user exists but password is wrong
                    // TODO: offer reset password option
                    Err(err)
                }
                _ => Err(err), // Propagate any other errors
            },
        }
    }

    pub fn set_admin_key(&mut self, new_key: CryptoKey) {
        let object_key = self.secure_storage.object_key().clone();
        self.secure_storage = SecureStorage::new(object_key, new_key);
    }

    pub async fn list_configurations(
        &self,
    ) -> SecureStringResult<HashMap<String, String>> {
        // Load the stored form metadata.
        let forms_db: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&self.secure_storage.load().await?)?;

        // Transform the metadata into the desired output format.
        let configurations = forms_db
            .into_iter()
            .map(|(id, meta)| {
                // Extract the name of the form, using "Unknown" as the default.
                let name = meta
                    .get("NAME")
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());

                // Return a tuple of the form id and name.
                (id, name)
            })
            .collect();

        Ok(configurations)
    }

    pub async fn save_secure_configuration(
        &mut self,
        object_key: ObjectKey,
        config: HashMap<String, String>,
    ) -> SecureStringResult<()> {
        let form_name = config
            .get("__NAME__")
            .unwrap_or(&"Unknown".to_string())
            .clone();
        let form_id = &object_key.id.clone();
        let password = generate_password()?;
        let derived_key = derive_crypto_key(&password, form_id).await?;
        let config_json = serde_json::to_string(&config)?;

        let secure_storage =
            SecureStorage::new(object_key.clone(), derived_key);
        secure_storage.save(&config_json).await?;

        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match self.secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };
        let mut form_config = HashMap::new();
        form_config.insert("NAME".to_string(), form_name);
        form_config.insert("PASSWD".to_string(), password);
        forms_db.insert(form_id.to_string(), form_config);
        self.secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await
    }

    // Loads the configuration securely after decrypting it with a derived key
    pub async fn load_secure_configuration(
        &self,
        object_key: ObjectKey,
    ) -> SecureStringResult<HashMap<String, String>> {
        let object_id = &object_key.id.clone();
        let meta: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&self.secure_storage.load().await?)?;
        let meta =
            meta.get(object_id)
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    object_id
                )))?;
        let password =
            meta.get("PASSWD")
                .ok_or(SecureStringError::PasswordNotFound(format!(
                    "Password for {} not found",
                    object_id
                )))?;

        let derived_key = derive_crypto_key(&password, object_id).await?;
        let secure_storage =
            SecureStorage::new(object_key.clone(), derived_key);
        let config_json = secure_storage.load().await?;
        let config: HashMap<String, String> =
            serde_json::from_str(&config_json)
                .map_err(SecureStringError::from)?;
        Ok(config)
    }

    pub async fn add_configuration(
        &mut self,
        object_key: ObjectKey,
        name: String,
    ) -> SecureStringResult<()> {
        let form_id = &object_key.id.clone();
        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match self.secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };
        let form_config = forms_db
            .entry(form_id.to_string())
            .or_insert_with(HashMap::new);
        form_config.insert("NAME".to_string(), name);
        self.secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await?;
        Ok(())
    }

    pub async fn delete_configuration(
        &mut self,
        object_key: ObjectKey,
    ) -> SecureStringResult<()> {
        let form_id = &object_key.id.clone();

        // Load the stored form metadata.
        let mut forms_db: HashMap<String, HashMap<String, String>> =
            match self.secure_storage.load().await {
                Ok(passwords_json) => serde_json::from_str(&passwords_json)?,
                Err(_) => HashMap::new(),
            };

        // Remove the specific configuration
        if forms_db.remove(form_id).is_none() {
            return Err(SecureStringError::PasswordNotFound(format!(
                "Configuration for {} not found",
                form_id
            )));
        }

        // Save the updated form metadata back to the vault.
        self.secure_storage
            .save(&serde_json::to_string(&forms_db)?)
            .await?;

        Ok(())
    }

    pub async fn reset_vault(username: &str) -> SecureStringResult<()> {
        let hashed_username = hash_username(username);
        let object_key = ObjectKey {
            tag: hashed_username.clone(),
            id: "self".to_string(),
        };

        let secure_storage = SecureStorage::for_deletion(object_key);
        secure_storage.delete().await?;

        Ok(())
    }
}
