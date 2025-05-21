use std::sync::Arc;
use std::time::Duration;

use html_form_actions::BuildExt;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    values: Arc<Mutex<Vec<i32>>>,
}

struct Values(tokio::sync::OwnedMutexGuard<Vec<i32>>);

impl<'r> picoserve::extract::FromRequestParts<'r, AppState> for Values {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        state: &'r AppState,
        _request_parts: &picoserve::request::RequestParts<'r>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Values(state.values.clone().lock_owned().await))
    }
}

#[html_form_actions::actions(picoserve, state = AppState)]
mod index_page {
    use picoserve::response::Redirect;

    use super::{AppState, Values};

    async fn page_handler(Values(values): Values) -> impl picoserve::response::IntoResponse {
        let body = maud::html! {
            (maud::DOCTYPE)
            html lang="en" {
                head {
                    meta charset="UTF-8";
                    meta name="viewport" content="width=device-width, initial-scale=1.0";
                    title { "Picoserve Example" }
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
        };

        struct Body(String);

        impl picoserve::response::Content for Body {
            fn content_type(&self) -> &'static str {
                "text/html; charset=utf-8"
            }

            fn content_length(&self) -> usize {
                self.0.len()
            }

            async fn write_content<W: picoserve::io::Write>(
                self,
                mut writer: W,
            ) -> Result<(), W::Error> {
                writer.write_all(self.0.as_bytes()).await
            }
        }

        Body(body.into_string())
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

    pub fn route<R: picoserve::routing::PathRouter<AppState>>(
        router: picoserve::Router<R, AppState>,
    ) -> picoserve::Router<impl picoserve::routing::PathRouter<AppState>, AppState> {
        router.route(
            "/",
            picoserve::routing::get(page_handler).post_service(ActionsHandler),
        )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let port = 8000;

    let app = std::rc::Rc::new(picoserve::Router::new().with(index_page::route));

    let state = AppState {
        values: Arc::new(Mutex::new(Vec::new())),
    };

    let config = picoserve::Config::new(picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        persistent_start_read_request: Some(Duration::from_secs(1)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    })
    .keep_connection_alive();

    let socket = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();

    println!("http://localhost:{port}/");

    tokio::task::LocalSet::new()
        .run_until(async {
            loop {
                let (stream, remote_address) = socket.accept().await.unwrap();

                println!("Connection from {remote_address}");

                let app = app.clone();
                let state = state.clone();
                let config = config.clone();

                tokio::task::spawn_local(async move {
                    match picoserve::serve_with_state(&app, &config, &mut [0; 2048], stream, &state)
                        .await
                    {
                        Ok(handled_requests_count) => {
                            println!(
                                "{handled_requests_count} requests handled from {remote_address}"
                            )
                        }
                        Err(err) => println!("{err:?}"),
                    }
                });
            }
        })
        .await
}
