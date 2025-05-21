use crate::{BuildExt, actions};
#[test]
fn basic() {
    #[actions(picoserve)]
    mod page {
        const PATH: &str = "/basic";

        async fn page_handler() -> impl picoserve::response::IntoResponse {
            (
                ("Content-Type", "text/html; charset=utf-8"),
                maud::html! {
                    @let my_action::Form { action, a_name } = my_action::FORM;
                    form action=(action) {
                        label { "A" input name=(a_name); }
                    }
                }
                .into_string(),
            )
        }

        #[action]
        async fn my_action(#[form] a: i32) -> String {
            std::format!("a = {a}")
        }

        pub fn route<R: picoserve::routing::PathRouter>(
            router: picoserve::Router<R>,
        ) -> picoserve::Router<impl picoserve::routing::PathRouter> {
            router.route(
                PATH,
                picoserve::routing::get(page_handler).post_service(ActionsHandler),
            )
        }
    }

    picoserve::Router::new().with(page::route);
}

#[test]
fn named_handler() {
    #[actions(picoserve(handler = NamedActionsHandler))]
    mod page {
        const PATH: &str = "/basic";

        async fn page_handler() -> impl picoserve::response::IntoResponse {
            (
                ("Content-Type", "text/html; charset=utf-8"),
                maud::html! {
                    @let my_action::Form { action, a_name } = my_action::FORM;
                    form action=(action) {
                        label { "A" input name=(a_name); }
                    }
                }
                .into_string(),
            )
        }

        #[action]
        async fn my_action(#[form] a: i32) -> String {
            std::format!("a = {a}")
        }

        pub fn route<R: picoserve::routing::PathRouter>(
            router: picoserve::Router<R>,
        ) -> picoserve::Router<impl picoserve::routing::PathRouter> {
            router.route(
                PATH,
                picoserve::routing::get(page_handler).post_service(NamedActionsHandler),
            )
        }
    }

    picoserve::Router::new().with(page::route);
}
