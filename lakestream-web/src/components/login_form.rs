use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;
use leptos_router::use_navigate;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::{GlobalState, StringVault};

const ROOT_USERNAME: &str = "root";

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    let set_vault =
        create_write_slice(cx, state, |state, vault| state.vault = vault);

    let previous_url =
        create_read_slice(cx, state, |state| state.previous_url.clone());

    let password_ref: NodeRef<Input> = create_node_ref(cx);

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        let password = password_ref().expect("password to exist").value();

        let redirect_url = previous_url();
        spawn_local(async move {
            match StringVault::new(ROOT_USERNAME, &password).await {
                Ok(string_vault) => {
                    set_vault(Some(string_vault));
                    let navigate = use_navigate(cx);
                    if let Err(e) = navigate(&redirect_url, Default::default()) {
                        log!("Error navigating to {}: {}", redirect_url, e);
                    }

                }
                Err(err) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Error deriving key: {:?}",
                        err
                    )));
                }
            }
        });
    };

    view! { cx,
        <form class="flex flex-col w-96"  on:submit=on_submit>
            <div class="flex flex-col mb-4">
                <label class="mb-2">"Password"</label>
                <input type="password"
                    class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                    node_ref=password_ref
                />
            </div>

            <button
                type="submit"
                class="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
            >
                "Log In"
            </button>
        </form>
    }
}
