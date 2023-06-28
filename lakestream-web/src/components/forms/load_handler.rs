use std::collections::HashMap;
use std::rc::Rc;

use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt, SecureStringError};

use super::form_data::FormData;
use super::handler::FormHandlerTrait;
use super::html_form::{Form, HtmlForm};
use super::submit_handler::SubmitHandler;
use super::view_handler::ViewHandler;
use crate::builders::LoadParameters;
use crate::components::form_input::FormElement;

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";
const CANT_LOAD_CONFIG: &str =
    "Can't load existing configuration. Creating new.";

pub trait LoadHandler {
    fn is_loading(&self) -> RwSignal<bool>;
    fn load_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormData>>;
}

pub struct LoadFormHandler {
    on_load: Box<dyn LoadHandler>,
}

impl LoadFormHandler {
    pub fn is_loading(&self) -> RwSignal<bool> {
        self.on_load.is_loading()
    }

    pub fn load_error(&self) -> RwSignal<Option<String>> {
        self.on_load.load_error()
    }

    pub fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.on_load.form_data()
    }
}

pub struct LoadForm {
    cx: Scope,
    form_handler: Rc<dyn FormHandlerTrait>,
}

impl LoadForm {
    pub fn new(form: HtmlForm, parameters: Option<LoadParameters>) -> Self {
        if let Some(parameters) = parameters {
            if let Some(handler) = parameters.load_handler {
                // load handler writes to form_data_rw
                handler(form.form_data_rw());
            }
        }
        let cx = form.cx();
        let handler: Box<DirectLoadHandler> = DirectLoadHandler::new(cx, form);
        let form_handler: Rc<dyn FormHandlerTrait> = Rc::new(*handler);
        Self {
            cx,
            form_handler,
        }
    }
}

impl Form for LoadForm {
    fn is_processing(&self) -> RwSignal<bool> {
        self.form_handler.is_processing()
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.form_handler.process_error()
    }

    fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form_handler.form_data()
    }

    fn to_view(&self) -> View {
        ViewHandler::new(Rc::clone(&self.form_handler)).to_view(self.cx, None)
    }
}

impl FormHandlerTrait for LoadFormHandler {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_loading()
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.load_error()
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data()
    }

    fn on_submit(&self) -> &dyn SubmitHandler {
        panic!("LoadFormHandler does not have a submit handler")
    }
}
pub struct LoadVaultHandler {
    form_data: RwSignal<Option<FormData>>,
    is_loading: RwSignal<bool>,
    load_error: RwSignal<Option<String>>,
}

impl LoadHandler for LoadVaultHandler {
    fn is_loading(&self) -> RwSignal<bool> {
        self.is_loading
    }

    fn load_error(&self) -> RwSignal<Option<String>> {
        self.load_error
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }
}

impl LoadVaultHandler {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Box<Self> {
        let is_loading = create_rw_signal(cx, true);
        let load_error = create_rw_signal(cx, None::<String>);
        let form_data = create_rw_signal(cx, None::<FormData>);

        let form = Rc::new(form);
        let vault_clone = vault.clone();

        spawn_local(async move {
            match get_form_data_from_vault(cx, &form, &vault_clone).await {
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
    content: Result<Option<Vec<u8>>, SecureStringError>,
) -> Result<FormData, String> {
    match content {
        Ok(data) => match data {
            Some(data) => {
                match serde_json::from_slice::<HashMap<String, String>>(&data) {
                    Ok(new_config) => {
                        let form_submit_data = FormData::build_with_config(
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
                }
            }
            None => {
                log::info!(
                    "No data found for the given form id: {}. Creating new.",
                    form_name
                );
                let form_submit_data =
                    FormData::build(cx, meta_data, form_elements);
                Ok(form_submit_data)
            }
        },
        Err(e) => match e {
            SecureStringError::PasswordNotFound(_)
            | SecureStringError::NoLocalStorageData => {
                log::info!("{} Creating new.", CANT_LOAD_CONFIG);
                let form_submit_data =
                    FormData::build(cx, meta_data, form_elements);
                Ok(form_submit_data)
            }
            _ => {
                log::error!("error loading config: {:?}", e);
                Err(e.to_string())
            }
        },
    }
}

pub async fn get_form_data_from_vault(
    cx: Scope,
    form: &HtmlForm,
    vault: &LocalEncrypt,
) -> Result<FormData, String> {
    let form_elements = form.elements.clone();
    let form_name = form.id(); // use id as name

    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), form.name().to_string());
    let meta_data = ItemMetaData::new_with_tags(form_name, tags);

    let content = load_form_data_from_vault(form_name, vault).await;
    handle_loaded_content(cx, form_name, &form_elements, meta_data, content)
}

async fn load_form_data_from_vault(
    form_name: &str,
    vault: &LocalEncrypt,
) -> Result<Option<Vec<u8>>, SecureStringError> {
    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    local_storage.load_content(form_name).await
}

pub struct DirectLoadHandler {
    is_loading: RwSignal<bool>,
    load_error: RwSignal<Option<String>>,
    form_data_rw: RwSignal<Option<FormData>>,
}

impl DirectLoadHandler {
    pub fn new(cx: Scope, form: HtmlForm) -> Box<Self> {
        let is_loading = create_rw_signal(cx, false);
        let load_error = create_rw_signal(cx, None::<String>);
        let form_data_rw = form.form_data_rw();

        Box::new(Self {
            is_loading,
            load_error,
            form_data_rw,
        })
    }
}

impl LoadHandler for DirectLoadHandler {
    fn is_loading(&self) -> RwSignal<bool> {
        self.is_loading
    }

    fn load_error(&self) -> RwSignal<Option<String>> {
        self.load_error
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data_rw
    }
}

impl FormHandlerTrait for DirectLoadHandler {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_loading()
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.load_error()
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data_rw
    }

    fn on_submit(&self) -> &dyn SubmitHandler {
        panic!(
            "DirectLoadHandler might not have a submit handler, handle this \
             case appropriately"
        )
    }
}
