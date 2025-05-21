use quote::ToTokens;
use syn::spanned::Spanned;

fn maybe_extract_attributes<T: deluxe::HasAttributes, R: deluxe::ExtractAttributes<T>>(
    obj: &mut T,
) -> deluxe::Result<Option<R>> {
    obj.attrs()
        .iter()
        .any(|attribute| R::path_matches(attribute.path()))
        .then(|| deluxe::extract_attributes(obj))
        .transpose()
}

struct ActionFormInput {
    ident: syn::Ident,
    rename: Option<syn::Expr>,
    form_name: syn::Ident,
    ty: syn::Type,
}

struct Action {
    ident: syn::Ident,
    form: Vec<ActionFormInput>,
    other_arguments: Vec<syn::Ident>,
}

impl Action {
    fn extract(items: &mut [syn::Item]) -> syn::Result<Vec<Self>> {
        #[derive(deluxe::ExtractAttributes)]
        #[deluxe(attributes(action))]
        struct ActionAttribute {}

        let mut actions = Vec::new();

        for item in items {
            let syn::Item::Fn(f) = item else {
                continue;
            };

            let Some(ActionAttribute {}) = maybe_extract_attributes(f)? else {
                continue;
            };

            let mut form = Vec::new();
            let mut other_arguments = Vec::new();

            for (index, input) in f.sig.inputs.iter_mut().enumerate() {
                match input {
                    syn::FnArg::Receiver(_) => {
                        return Err(syn::Error::new(input.span(), r#""self" is not allowed"#));
                    }
                    syn::FnArg::Typed(syn::PatType {
                        attrs,
                        pat,
                        colon_token: _,
                        ty,
                    }) => {
                        #[derive(deluxe::ExtractAttributes)]
                        struct FormAttrs {
                            #[deluxe(default)]
                            rename: Option<syn::Expr>,
                        }

                        fn pat_ident(pat: &syn::Pat) -> Option<syn::Ident> {
                            if let syn::Pat::Ident(syn::PatIdent { ident, .. }) = pat {
                                Some(ident.clone())
                            } else {
                                None
                            }
                        }

                        if let Some(FormAttrs { rename }) = maybe_extract_attributes(attrs)? {
                            let ident = pat_ident(pat).ok_or_else(|| {
                                syn::Error::new(
                                    pat.span(),
                                    "parameters tagged with #[form] must be identifiers",
                                )
                            })?;

                            let form_name = syn::Ident::new(&format!("{ident}_name"), ident.span());

                            form.push(ActionFormInput {
                                ident,
                                rename,
                                form_name,
                                ty: ty.as_ref().clone(),
                            });
                        } else {
                            other_arguments.push(pat_ident(pat).unwrap_or_else(|| {
                                syn::Ident::new(&format!("arg_{index}"), pat.span())
                            }))
                        }
                    }
                }
            }

            actions.push(Action {
                ident: f.sig.ident.clone(),
                form,
                other_arguments,
            });
        }

        Ok(actions)
    }

    fn query(&self) -> String {
        format!("/{}", self.ident)
    }

    fn struct_declaration(&self) -> proc_macro2::TokenStream {
        let form_fields = self.form.iter().map(
            |ActionFormInput {
                 ident,
                 rename,
                 form_name: _,
                 ty,
             }| {
                let rename = rename
                    .as_ref()
                    .map(|name| quote::quote! { #[serde(rename = #name)] });

                quote::quote! { #rename #ident: #ty }
            },
        );

        quote::quote! {
            #[derive(serde::Deserialize)]
            struct Form {
                #(#form_fields,)*
            }
        }
    }
}

#[derive(deluxe::ParseMetaItem)]
struct AxumActionAttributes {
    #[deluxe(default = syn::Ident::new("actions_handler", proc_macro2::Span::call_site()))]
    handler: syn::Ident,
}

#[derive(deluxe::ParseMetaItem)]
struct PicoserveActionAttributes {
    #[deluxe(default)]
    path_parameters: Vec<syn::Type>,
    #[deluxe(default = syn::Ident::new("ActionsHandler", proc_macro2::Span::call_site()))]
    handler: syn::Ident,
}

mod optional_struct {
    pub fn parse_meta_item_named<T: deluxe::ParseMetaItem>(
        input: syn::parse::ParseStream,
        _name: &str,
        span: proc_macro2::Span,
    ) -> deluxe::Result<Option<T>> {
        deluxe_core::parse_helpers::parse_named_meta_item(input, span).map(Some)
    }
}

#[derive(deluxe::ParseMetaItem)]
struct ActionAttributes {
    #[deluxe(default)]
    state: Option<syn::Type>,
    #[deluxe(default, with = optional_struct)]
    axum: Option<AxumActionAttributes>,
    #[deluxe(default, with = optional_struct)]
    picoserve: Option<PicoserveActionAttributes>,
}

fn axum_handler(
    state: &Option<syn::Type>,
    AxumActionAttributes { handler }: AxumActionAttributes,
    actions: &[Action],
) -> syn::Result<syn::ItemFn> {
    let state_argument = state.as_ref().map(|state| {
        quote::quote! {
            axum::extract::State(state): axum::extract::State<#state>,
        }
    });

    let action_cases = actions.iter().map(
        |action @ Action {
             ident,
             form,
             other_arguments,
         }| {
            let query = action.query();

            let struct_declaration = action.struct_declaration();

            let form_field_names = form
                .iter()
                .map(|ActionFormInput { ident, .. }| ident)
                .collect::<Vec<_>>();

            let action_call = quote::quote! {
                |#(#other_arguments,)* Form(Form { #(#form_field_names,)* })| async move {
                    #ident ( #(#form_field_names,)* #(#other_arguments,)*).await.into_response()
                }
            };

            let state_value = if state.is_some() {
                quote::quote! { state }
            } else {
                quote::quote! { () }
            };

            quote::quote! {
                Some(#query) => {
                    #struct_declaration

                    Handler::call(
                        #action_call,
                        request,
                        #state_value,
                    )
                    .await
                },
            }
        },
    );

    Ok(syn::parse_quote! {
        async fn #handler(
            #state_argument
            axum::extract::RawQuery(query): axum::extract::RawQuery,
            request: axum::extract::Request,
        ) -> axum::response::Response {
            use axum::{extract::Form, handler::Handler, response::IntoResponse};

            match html_form_actions::query_action(query.as_deref()) {
                #(#action_cases)*
                _ => (axum::http::StatusCode::NOT_FOUND, "Action Not Found").into_response(),
            }
        }
    })
}

fn picoserve_handler(
    state: &Option<syn::Type>,
    PicoserveActionAttributes {
        path_parameters,
        handler,
    }: PicoserveActionAttributes,
    actions: &[Action],
) -> (syn::ItemStruct, syn::ItemImpl) {
    let generic_state_name = quote::quote! {State};

    let state_generics = state
        .is_none()
        .then(|| quote::quote! { < #generic_state_name > });

    let state = state
        .as_ref()
        .map_or(generic_state_name, ToTokens::into_token_stream);

    let path_parameter_names = path_parameters
        .iter()
        .enumerate()
        .map(|(index, ty)| syn::Ident::new(&format!("path_parameter_{index}"), ty.span()))
        .collect::<Vec<_>>();

    let action_cases = actions.iter().map(
        |action @ Action {
             ident,
             form,
             other_arguments,
         }| {
            let query = action.query();

            let struct_declaration = action.struct_declaration();

            let form_field_names = form
                .iter()
                .map(|ActionFormInput { ident, .. }| ident)
                .collect::<Vec<_>>();

            let action_call = quote::quote! {
                |#(#other_arguments,)* Form(Form { #(#form_field_names,)* })| async move  {
                    #ident ( #(#form_field_names,)* #(#other_arguments,)*).await
                }
            };

            let path_parameter_list = match path_parameter_names.as_slice() {
                [] => quote::quote! { picoserve::routing::NoPathParameters },
                [name] => quote::quote! { picoserve::routing::OnePathParameter(#name) },
                list => quote::quote! { picoserve::routing::ManyPathParameters((#(#list,)*)) },
            };

            quote::quote! {
                Some(query) if query == #query => {
                    #struct_declaration

                    picoserve::routing::RequestHandlerFunction::call_handler_func(
                        &#action_call,
                        state,
                        #path_parameter_list,
                        request,
                        response_writer,
                    )
                    .await
                }
            }
        },
    );

    let impl_item = syn::parse_quote! {
        impl #state_generics picoserve::routing::RequestHandlerService<#state, (#(#path_parameters,)*)> for #handler {
            async fn call_request_handler_service<
                R: picoserve::io::Read,
                W: picoserve::response::ResponseWriter<Error = R::Error>,
            >(
                &self,
                state: &#state,
                (#(#path_parameter_names,)*) : (#(#path_parameters,)*),
                request: picoserve::request::Request<'_, R>,
                response_writer: W,
            ) -> Result<picoserve::ResponseSent, W::Error> {
                use picoserve::{extract::Form, response::IntoResponse};


                match request.parts.query() {
                    #(#action_cases)*
                    _ => {
                        (
                            picoserve::response::StatusCode::NOT_FOUND,
                            "Action Not Found",
                        )
                            .write_to(request.body_connection.finalize().await?, response_writer)
                            .await
                    }
                }
            }
        }
    };

    (syn::parse_quote! { struct #handler; }, impl_item)
}

fn try_actions(
    attribute_tokens: proc_macro::TokenStream,
    tokens: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let ActionAttributes {
        state,
        axum,
        picoserve,
    } = deluxe::parse(attribute_tokens)?;

    let module = syn::parse(tokens)?;

    let syn::ItemMod {
        attrs,
        vis,
        unsafety,
        mod_token,
        ident,
        content: Some((brace, mut items)),
        semi,
    } = module
    else {
        return Err(syn::Error::new(
            module.span(),
            "The module must have content",
        ));
    };

    let actions = Action::extract(&mut items)?;

    let action_modules = actions.iter().map(
        |Action {
             ident,
             form,
             other_arguments: _,
         }| {
            let action = format!("?/{ident}");

            let form_struct_field_definitions = form.iter().map(
                |ActionFormInput { form_name, .. }| quote::quote! { pub(super) #form_name: &'static str },
            );

            let form_struct_field_declarations = form.iter().map(
                |ActionFormInput {
                     ident,
                     rename,
                     form_name,
                     ty: _,
                 }| {
                    let name = rename.as_ref().map_or_else(
                        || ident.to_string().to_token_stream(),
                        quote::ToTokens::to_token_stream,
                    );

                    quote::quote! { #form_name: #name }
                },
            );

            syn::Item::Mod(syn::parse_quote! {
                mod #ident {
                    pub(super)struct Form {
                        pub(super) action: &'static str,
                        #(#form_struct_field_definitions,)*
                    }

                    pub(super) const FORM: Form = Form {
                        action: #action,
                        #(#form_struct_field_declarations,)*
                    };
                }
            })
        },
    );

    items.extend(action_modules);

    if let Some(axum) = axum {
        items.push(syn::Item::Fn(axum_handler(&state, axum, &actions)?));
    }

    if let Some(picoserve) = picoserve {
        let (service, service_impl) = picoserve_handler(&state, picoserve, &actions);

        items.extend([syn::Item::Struct(service), syn::Item::Impl(service_impl)]);
    }

    Ok(syn::ItemMod {
        attrs,
        vis,
        unsafety,
        mod_token,
        ident,
        content: Some((brace, items)),
        semi,
    }
    .to_token_stream()
    .into())
}

/// Declare a module as containing form action handlers.
///
/// Action handlers are functions which handle form actions, typically over POST.
///
/// To declare a function as a handler, annotate it with the `#[action]` attribute.
///
/// Function parameters representing form fields should be annotated with the `#[form]` attribute.
///
/// # Attributes
///
/// - `state` - The "state" used in generated handlers.
/// - `axum` - Integrate with [`axum`](https://docs.rs/axum).
///   - `handler` - The name of the generated handler to be used as the POST handler. Defaults to `actions_handler`.
/// - `picoserve` - Integrate with [`picoserve`](https://docs.rs/picoserve).
///   - `path_parameters` - The types of the path parameters.
///   - `handler` - The name of the generated struct which implements [`RequestHandlerService`](https://docs.rs/picoserve/latest/picoserve/routing/trait.RequestHandlerService.html).
///
/// # Macro Output
///
/// The macro modifies the module, inserting the following content:
///
/// - For each "action", a module with the same name is generated, containing:
///   - A `pub struct` called `Form` representing the form values, with the following field:
///     - `action` - The name of the action, to be used as the "action" attribute of the HTML form.
///     - For each `#[form]` parameter, `{parameter_name}_name` - The name of the form field, to be used as the "name" attribute of the HTML input.
///   - A `pub const` called `FORM`, containing the values of `Form`.
///
/// - If `axum` integration is declared:
///   - A function which can be used as an axum [`Handler`](https://docs.rs/axum/latest/axum/handler/index.html), which routes the request to the appropriate `#[action]`.
///
/// - If `picoserve` integration is declared:
///   - A struct which implements [`RequestHandlerService`](https://docs.rs/picoserve/latest/picoserve/routing/trait.RequestHandlerService.html) by routing the request to the appropriate `#[action]`.
///
/// All other content is unchanged, allowing you to mix action handlers with other items.
#[proc_macro_attribute]
pub fn actions(
    attribute_tokens: proc_macro::TokenStream,
    tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    try_actions(attribute_tokens, tokens)
        .map_or_else(|error| error.into_compile_error().into(), From::from)
}
