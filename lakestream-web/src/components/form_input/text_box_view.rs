use leptos::*;

use super::{DisplayValue, ElementDataType, FormElementState};
use crate::components::icons::LockIconView;

const MASKED_VALUE: &str = "*****";

#[component]
pub fn TextBoxView(
    cx: Scope,
    form_element_state: FormElementState,
    input_changed: RwSignal<bool>,
) -> impl IntoView {
    // shows Label, InputField and Error
    let value_signal = form_element_state.display_value;
    let error_signal = form_element_state.display_error;
    let input_field_data = form_element_state.schema;

    let element_type = &input_field_data.element_type;
    let initial_enabled = input_field_data.is_enabled;
    let (label_text, is_secret, is_password) =
        if let ElementDataType::TextData(text_data) = element_type {
            let label_text = text_data
                .field_label
                .as_ref()
                .map_or_else(String::new, |label| label.text());
            let is_secret = text_data.field_type.is_secret();
            let is_password = text_data.field_type.is_password();
            (label_text, is_secret, is_password)
        } else {
            // Handle other cases for BinaryData, DocumentData, etc. or panic
            panic!("Not yet implemented");
        };

    // show lock icon if secret and not password (passwords cant be unlocked)
    let show_lock_icon = is_secret && initial_enabled && !is_password;

    // signals
    let initial_value = value_signal.get();
    let is_locked = create_rw_signal(
        cx,
        if initial_value.is_empty() {
            false
        } else {
            is_secret || is_password
        },
    );
    let is_enabled = (move || {
        if is_locked.get() {
            false
        } else {
            initial_enabled
        }
    })
    .derive_signal(cx);

    let initial_value = if is_locked.get() {
        match initial_value {
            DisplayValue::Text(text) => {
                if text.is_empty() {
                    "".to_string()
                } else {
                    MASKED_VALUE.to_string()
                }
            }
            DisplayValue::Binary(_) => MASKED_VALUE.to_string(),
        }
    } else {
        match initial_value {
            DisplayValue::Text(text) => text,
            DisplayValue::Binary(_) => "".to_string(),
        }
    };

    let display_value_signal = create_rw_signal(cx, initial_value);

    let click_handler: Box<dyn Fn()> = Box::new(move || {
        let new_state = !is_locked.get();
        let current_value = value_signal.get();
        is_locked.set(new_state);
        display_value_signal.set(if new_state {
            MASKED_VALUE.to_string()
        } else {
            match current_value {
                DisplayValue::Text(t) => t,
                DisplayValue::Binary(_) => "".to_string(),
            }
        });
    });

    let icon_view: View = if show_lock_icon {
        view! {
            cx,
            <div class="w-8">
                <LockIconView
                    is_locked
                    click_handler
                />
            </div>
        }
        .into_view(cx)
    } else {
        view! { cx, }.into_view(cx)
    };

    view! {
        cx,
        <div class="w-full flex-col items-start text-left mb-2 p-2 bg-white text-gray-800">
            <InputFieldLabelView
                label_text
                icon_view=icon_view
            />
            <InputFieldView
                is_password
                is_enabled
                value_signal
                display_value_signal
                input_changed
            />
            <InputFieldErrorView error_signal/>
        </div>
    }
}

#[component]
pub fn InputFieldLabelView(
    cx: Scope,
    label_text: String,
    icon_view: View,
) -> impl IntoView {
    view! {
        cx,
        <div class="flex justify-between items-center">
            <label for="field_id" class="text-base font-semibold text-gray-900">{label_text}</label>
            {icon_view}
        </div>

    }
}

#[component]
pub fn InputFieldView(
    cx: Scope,
    is_password: bool,
    is_enabled: Signal<bool>,
    value_signal: RwSignal<DisplayValue>,
    display_value_signal: RwSignal<String>,
    input_changed: RwSignal<bool>,
) -> impl IntoView {
    view! { cx,
        <input
            type=if is_password { "password" } else { "text" }
            prop:value= { display_value_signal }
            on:input=move |ev| {
                if is_enabled.get() {
                    let value = event_target_value(&ev);
                    value_signal.set(DisplayValue::Text(value));
                    input_changed.set(true);    // enable submit button
                }
            }
            placeholder="none".to_string()
            class=move || {format!("{} w-full", get_input_class(is_enabled.get()))}
            disabled=move || { !is_enabled.get() }
        />
    }
}

fn get_input_class(is_enabled: bool) -> &'static str {
    if is_enabled {
        "bg-gray-50 border border-gray-300 text-gray-900 rounded-lg \
         focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5"
    } else {
        "bg-gray-50 border border-gray-300 text-gray-900 rounded-lg block \
         w-full p-2.5"
    }
}

#[component]
pub fn InputFieldErrorView(
    cx: Scope,
    error_signal: RwSignal<Option<String>>,
) -> impl IntoView {
    view! { cx,
        <div class="text-red-500">
            { move || error_signal.get().unwrap_or("".to_string()) }
        </div>
    }
    .into_view(cx)
}
