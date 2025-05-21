use crate::{BuildExt, actions};

#[test]
fn basic() {
    #[actions(axum)]
    mod page {
        use axum::routing::get;

        use crate as html_form_actions;

        const PATH: &str = "/basic";

        async fn page_handler() -> maud::PreEscaped<String> {
            maud::html! {
                @let my_action::Form { action, a_name } = my_action::FORM;
                form action=(action) {
                    label { "A" input name=(a_name); }
                }
            }
        }

        #[action]
        async fn my_action(#[form] a: i32) -> String {
            std::format!("a = {a}")
        }

        pub fn route(router: axum::Router) -> axum::Router {
            router.route(PATH, get(page_handler).post(actions_handler))
        }
    }

    axum::Router::new().with(page::route).into_make_service();
}

#[test]
fn state() {
    #[actions(axum, state = AppState)]
    mod page {
        use axum::{extract::State, routing::get};

        use crate as html_form_actions;

        #[derive(Clone)]
        pub struct AppState {
            pub value: i32,
        }

        const PATH: &str = "/basic";

        async fn page_handler() -> maud::PreEscaped<String> {
            maud::html! {
                @let my_action::Form { action, a_name } = my_action::FORM;
                form action=(action) {
                    label { "A" input name=(a_name); }
                }
            }
        }

        #[action]
        async fn my_action(#[form] a: i32, State(AppState { value }): State<AppState>) -> String {
            std::format!("a = {a}, value = {value}")
        }

        pub fn route(router: axum::Router<AppState>) -> axum::Router<AppState> {
            router.route(PATH, get(page_handler).post(actions_handler))
        }
    }

    axum::Router::new()
        .with(page::route)
        .with_state(page::AppState { value: 42 })
        .into_make_service();
}

#[test]
fn named_handler() {
    #[actions(axum(handler = named_actions_handler))]
    mod page {
        use axum::routing::get;

        use crate as html_form_actions;

        const PATH: &str = "/basic";

        async fn page_handler() -> maud::PreEscaped<String> {
            maud::html! {
                @let my_action::Form { action, a_name } = my_action::FORM;
                form action=(action) {
                    label { "A" input name=(a_name); }
                }
            }
        }

        #[action]
        async fn my_action(#[form] a: i32) -> String {
            std::format!("a = {a}")
        }

        pub fn route(router: axum::Router) -> axum::Router {
            router.route(PATH, get(page_handler).post(named_actions_handler))
        }
    }

    axum::Router::new().with(page::route).into_make_service();
}
