use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::routes::{Home, Login, ObjectStores, ObjectStoresId};
use crate::{GlobalState, RunTime};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let state = create_rw_signal(cx, GlobalState::default());
    provide_context(cx, state);

    let set_previous_url =
        create_write_slice(cx, state, |state, previous_url| {
            state
                .runtime
                .get_or_insert_with(RunTime::new)
                .set_previous_url(previous_url);
        });

    let vault_initialized = create_read_slice(cx, state, |state| {
        state
            .runtime
            .as_ref()
            .map(|r| r.vault_initialized())
            .unwrap_or_default()
    });

    view! {
        cx,
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">

            <Router>
                <nav class="bg-teal-500 p-3">
                    <div class="mb-2 text-white text-2xl font-bold">"Lakestream"</div>
                    <div class="flex">
                        <a href="/home" class="text-teal-200 hover:text-white mr-4">"Home"</a>
                        <a href="/object-stores" class="text-teal-200 hover:text-white mr-4">"ObjectStores"</a>
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route path="/home" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/login" view=|cx| view! { cx, <Login/> }/>
                        <ProtectedRoute
                            path="/object-stores"
                            redirect_path="/login"
                            condition=move |_| {
                                log!("Checking move: {}", vault_initialized.get());
                                if !vault_initialized.get() {
                                    set_previous_url("/object-stores".to_string());
                                }
                                vault_initialized.get()
                            }
                            view=|cx| view! { cx, <ObjectStores/> }
                        />
                        <ProtectedRoute
                            path="/object-stores/:id"
                            redirect_path="/login"
                            condition=move |_| {
                                if !vault_initialized.get() {
                                    set_previous_url("/object-stores".to_string());
                                }
                                vault_initialized.get()
                            }
                            view=|cx| view! { cx, <ObjectStoresId/> }/>
                    </Routes>
                </main>
            </Router>

        </div>
    }
}
