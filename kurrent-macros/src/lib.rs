use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, AttrStyle, Attribute, FieldsNamed, Ident, Token, Visibility,
};

struct Structure {
    attrs: Vec<Attribute>,
    visibility: Visibility,
    _struct_token: Token![struct],
    name: Ident,
    fields: FieldsNamed,
}

impl Parse for Structure {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Structure {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: input.parse()?,
            _struct_token: input.parse()?,
            name: input.parse()?,
            fields: input.parse()?,
        })
    }
}

fn attr_has_streaming_name(attr: &Attribute) -> bool {
    attr.style == AttrStyle::Outer && attr.path.is_ident("streaming")
}

#[proc_macro_attribute]
pub fn streaming(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro]
pub fn options(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Structure);

    let fields = input.fields.named.iter().map(|field| {
        let attrs = if field.attrs.is_empty() {
            quote! {}
        } else {
            let attrs = field.attrs.iter().map(|attr| quote! { #attr });
            quote! {
                #(#attrs)*
            }
        };

        let vis = &field.vis;
        let name = &field.ident;
        let ty = &field.ty;

        quote! {
            #attrs
            #vis #name: #ty,
        }
    });

    let is_streaming = input.attrs.iter().any(attr_has_streaming_name);

    let kind = if is_streaming {
        quote! {
            crate::options::OperationKind::Streaming
        }
    } else {
        quote! {
            crate::options::OperationKind::Regular
        }
    };

    let attrs = input.attrs.iter().map(|attr| quote! { #attr });
    let vis = &input.visibility;
    let name = input.name;

    TokenStream::from(quote! {
        #(#attrs)*
        #vis struct #name {
            #(#fields)*
            pub(crate) common_operation_options: crate::options::CommonOperationOptions,
        }

        impl crate::options::Options for #name {
            fn common_operation_options(&self) -> &crate::options::CommonOperationOptions {
                &self.common_operation_options
            }

            fn kind(&self) -> crate::options::OperationKind {
                #kind
            }
        }

        impl #name {
            /// Performs the command with the given credentials.
            pub fn authenticated(mut self, credentials: crate::types::Credentials) -> Self {
                self.common_operation_options.credentials = Some(credentials);
                self
            }

            pub fn requires_leader(mut self, requires_leader: bool) -> Self {
                self.common_operation_options.requires_leader = requires_leader;
                self
            }

            pub fn deadline(mut self, deadline: std::time::Duration) -> Self {
                self.common_operation_options.deadline = Some(deadline);
                self
            }
        }
    })
}
