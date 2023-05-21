use std::cell::RefCell;
use std::rc::Rc;

use leptos::html::Input;
use leptos::*;
use uuid::Uuid;

use crate::base::{ObjectStore, ObjectStoreList};
use crate::{GlobalState, StringVault};

#[component]
pub fn ObjectStoreConfigurator(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let (item_list, set_item_list) =
        create_signal(cx, ObjectStoreList::new(vault.clone()));
    provide_context(cx, set_item_list);

    let input_ref = create_node_ref::<Input>(cx);

    fn get_input_value(input_ref: NodeRef<Input>) -> Option<String> {
        let input = input_ref.get()?;
        let value = input.value().trim().to_string();
        if !value.is_empty() {
            input.set_value("");
            Some(value)
        } else {
            None
        }
    }

    fn create_object_store(
        uri: String,
        vault: Rc<RefCell<StringVault>>,
    ) -> ObjectStore {
        ObjectStore::new(Uuid::new_v4(), uri, vault)
    }

    create_effect(cx, move |_| {
        item_list.get().save_to_local_storage();
    });

    create_effect(cx, move |_| {
        if let Some(input) = input_ref.get() {
            request_animation_frame(move || {
                let _ = input.focus();
            });
        }
    });

    let vault_clone = vault.clone();
    let input_ref_clone = input_ref.clone();
    view! { cx,
        <div>
            <input class="px-4 py-2"
                placeholder="Bucket URI"
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        if let Some(uri) = get_input_value(input_ref_clone.clone()) {
                            let new_item = create_object_store(uri, vault.clone());
                            set_item_list.update(|item_list| item_list.add(new_item));
                        }
                    }
                }
                node_ref=input_ref
            />
            <button class="px-4 py-2" on:click=move |_| {
                if let Some(uri) = get_input_value(input_ref_clone.clone()) {
                    let new_item = create_object_store(uri, vault_clone.clone());
                    set_item_list.update(|item_list| item_list.add(new_item));
                }
            }> "Add Item" </button>
        </div>
        <div>
            <ul>
                <For
                    each={move || item_list.get().items.clone()}
                    key=|item| item.id
                    view=move |cx, item: ObjectStore| view! { cx, <ListItem item /> }
                />
            </ul>
        </div>
    }
}

#[component]
fn ListItem(cx: Scope, item: ObjectStore) -> impl IntoView {
    let set_item = use_context::<WriteSignal<ObjectStoreList>>(cx).unwrap();
    let item_id = item.id;
    let item_uri = item.uri;

    view! { cx,
        <li>
            <div class="px-4 py-2">
                <a href={format!("/object-stores/{}", item_id)}>
                    {item_uri.clone()}
                </a>
                " | "
                <button class="text-red-500 hover:text-red-700" on:click=move |_| set_item.update(|t| t.remove(item_id))> "delete" </button>
            </div>
        </li>
    }
}