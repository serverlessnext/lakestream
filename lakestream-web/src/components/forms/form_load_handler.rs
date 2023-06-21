use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt, SecureStringError};

use super::{FormSubmitData, HtmlForm};
use crate::components::form_input::{FormElement, InputElements};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";
const CANT_LOAD_CONFIG: &str =
    "Can't load existing configuration. Creating new.";

pub trait FormLoadHandler {
    fn is_loading(&self) -> RwSignal<bool>;
    fn load_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormSubmitData>>;
}

pub struct FormLoadVaultHandler {
    form_data: RwSignal<Option<FormSubmitData>>,
    is_loading: RwSignal<bool>,
    load_error: RwSignal<Option<String>>,
}

impl FormLoadHandler for FormLoadVaultHandler {
    fn is_loading(&self) -> RwSignal<bool> {
        self.is_loading
    }

    fn load_error(&self) -> RwSignal<Option<String>> {
        self.load_error
    }

    fn form_data(&self) -> RwSignal<Option<FormSubmitData>> {
        self.form_data
    }
}

use std::rc::Rc;
impl FormLoadVaultHandler {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Box<Self> {
        let is_loading = create_rw_signal(cx, true);
        let load_error = create_rw_signal(cx, None::<String>);
        let form_data = create_rw_signal(cx, None::<FormSubmitData>);

        let form = Rc::new(form);
        let vault_clone = vault.clone();

        spawn_local(async move {
            match get_form_data(cx, &*form, &vault_clone).await {
                Ok(form_submit_data) => {
                    form_data.set(Some(form_submit_data));
                    is_loading.set(false);
                }
                Err(error) => {
                    load_error.set(Some(error));
                    is_loading.set(false);
                }
            }
        });

        Box::new(Self {
            form_data,
            is_loading,
            load_error,
        })
    }
}

fn handle_loaded_content(
    cx: Scope,
    form_name: &str,
    form_elements: &[FormElement],
    meta_data: ItemMetaData,
    content: Option<Vec<u8>>,
    default_field_values: &HashMap<String, String>,
) -> Result<FormSubmitData, String> {
    match content {
        Some(data) => match serde_json::from_slice(&data) {
            Ok(new_config) => {
                let form_submit_data = create_form_submit_data(
                    cx,
                    meta_data,
                    &new_config,
                    form_elements,
                );
                Ok(form_submit_data)
            }
            Err(e) => {
                log::error!("error deserializing config: {:?}", e);
                Err(e.to_string())
            }
        },
        None => {
            log::info!(
                "No data found for the given form id: {}. Creating new.",
                form_name
            );
            let form_submit_data = create_form_submit_data(
                cx,
                meta_data,
                default_field_values,
                form_elements,
            );
            Ok(form_submit_data)
        }
    }
}

pub async fn get_form_data(
    cx: Scope,
    form: &HtmlForm,
    vault: &LocalEncrypt,
) -> Result<FormSubmitData, String> {
    let default_field_values = form.default_field_values();
    let form_elements = form.elements();
    let form_name = form.id();

    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), form.name());
    let meta_data = ItemMetaData::new_with_tags(&form_name, tags);

    match local_storage.load_content(&form_name).await {
        Ok(content) => handle_loaded_content(
            cx,
            &form_name,
            &form_elements,
            meta_data,
            content,
            &default_field_values,
        ),
        Err(e) => match e {
            SecureStringError::PasswordNotFound(_)
            | SecureStringError::NoLocalStorageData => {
                log::info!("{} Creating new.", CANT_LOAD_CONFIG);
                let form_submit_data = create_form_submit_data(
                    cx,
                    meta_data,
                    &default_field_values,
                    &form_elements,
                );
                Ok(form_submit_data)
            }
            _ => {
                log::error!("error loading config: {:?}", e);
                Err(e.to_string())
            }
        },
    }
}

fn create_form_submit_data(
    cx: Scope,
    meta_data: ItemMetaData,
    config: &HashMap<String, String>,
    elements: &[FormElement],
) -> FormSubmitData {
    let input_elements: InputElements = config
        .iter()
        .filter_map(|(key, value)| {
            elements.iter().find_map(|element| match element {
                FormElement::InputField(field_data) => {
                    if field_data.name == *key {
                        let error_signal = create_rw_signal(cx, None);
                        let value_signal = create_rw_signal(cx, value.clone());
                        let default_input_data = field_data.clone();
                        Some((
                            key.clone(),
                            (
                                create_node_ref(cx),
                                error_signal,
                                value_signal,
                                Arc::new(default_input_data),
                            ),
                        ))
                    } else {
                        None
                    }
                }
            })
        })
        .collect();
    FormSubmitData::new(input_elements, meta_data)
}
