use std::sync::Arc;

use html_form_actions::BuildExt;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    values: Arc<Mutex<Vec<i32>>>,
}

struct Values(tokio::sync::OwnedMutexGuard<Vec<i32>>);

impl axum::extract::FromRequestParts<AppState> for Values {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Values(state.values.clone().lock_owned().await))
    }
}

#[html_form_actions::actions(axum, state = AppState)]
mod index_page {
    use axum::response::Redirect;

    use super::{AppState, Values};

    pub const PATH: &str = "/";

    async fn page_handler(Values(values): Values) -> impl axum::response::IntoResponse {
        maud::html! {
            (maud::DOCTYPE)
            html lang="en" {
                head {
                    meta charset="UTF-8";
                    meta name="viewport" content="width=device-width, initial-scale=1.0";
                    title { "Axum Example" }
                    style { (maud::PreEscaped(include_str!("style.css"))) }
                }
                body {
                    ul {
                        @for (index, value) in values.iter().enumerate() {
                            li {
                                fieldset {
                                    {
                                        @let update_value::Form { action, index_name, value_name } = update_value::FORM;
                                        form action=(action) method="post" {
                                            input type="hidden" name=(index_name) value=(index);
                                            input type="number" name=(value_name) value=(value);
                                            input type="submit" value="Update Value";
                                        }
                                    }
                                    {
                                        @let remove_value::Form { action, index_name } = remove_value::FORM;
                                        form action=(action) method="post" {
                                            input type="hidden" name=(index_name) value=(index);
                                            input type="submit" value="Remove Value";
                                        }
                                    }
                                }
                            }
                        }
                    }
                    {
                        @let add_value::Form { action, value_name } = add_value::FORM;
                        form action=(action) method="post" {
                            fieldset {
                                input type="number" name=(value_name) value="0";
                                input type="submit" value="Add Value";
                            }
                        }
                    }
                }
            }
        }
    }

    #[action]
    async fn add_value(#[form] value: i32, Values(mut values): Values) -> Redirect {
        values.push(value);
        Redirect::to("")
    }

    #[action]
    async fn update_value(
        #[form] index: usize,
        #[form] value: i32,
        Values(mut values): Values,
    ) -> Redirect {
        if let Some(slot) = values.get_mut(index) {
            *slot = value;
        }

        Redirect::to("")
    }

    #[action]
    async fn remove_value(#[form] index: usize, Values(mut values): Values) -> Redirect {
        if index < values.len() {
            values.remove(index);
        }

        Redirect::to("")
    }

    pub fn route(router: axum::Router<AppState>) -> axum::Router<AppState> {
        router.route(PATH, axum::routing::get(page_handler).post(actions_handler))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let app = axum::Router::new()
        .with(index_page::route)
        .with_state(AppState {
            values: Arc::new(Mutex::new(Vec::new())),
        });

    axum::serve(
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 8000))
            .await
            .unwrap(),
        app,
    )
    .await
    .unwrap();
}
