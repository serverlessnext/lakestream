use leptos::*;

use super::list_view::ConfigurationListView;

#[component]
pub fn Configurations(cx: Scope) -> impl IntoView {
    view! { cx,
        <ConfigurationListView />
    }
}
