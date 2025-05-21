# html_form_actions

Generate HTML Form parsers and routing logic for Form Actions.

The main usage is the `actions` proc-macro, which allows you to declare a module of "Action" handlers.

Action Handlers, which are declared as such with the `#[action]` attribute, may have parameters with the `#[form]` attribute, which generates a structure which describes the form structure, allowing code to use them in template code to ensure that the HTML form and the parsing logic matches.

See the [docs](https://docs.rs/axum/html_form_actions) for more info.

## Features

- `axum` will enable integration with [`axum`](https://docs.rs/axum), generating a function which can be used as an axum [`Handler`](https://docs.rs/axum/latest/axum/handler/index.html), which routes the request to the appropriate `#[action]`.
- `picoserve` will enable integration with [`picoserve`](https://docs.rs/picoserve), generating a struct which implements [`RequestHandlerService`](https://docs.rs/picoserve/latest/picoserve/routing/trait.RequestHandlerService.html) by routing the request to the appropriate `#[action]`.

