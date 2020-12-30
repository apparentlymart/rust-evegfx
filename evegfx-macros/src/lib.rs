extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::parse_macro_input;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, ExprPath, Lit, LitStr, Token};

mod parsers;

/// Prepare a format string and associated arguments for use with an EVE
/// coprocessor widget which supports the `OPT_FORMAT` option.
#[proc_macro]
pub fn eve_format(input: TokenStream) -> TokenStream {
    let call = parse_macro_input!(input as EVEFormat);
    let fmt = call.fmt;
    let args = call.args;

    let fmt_src = &fmt.value().into_bytes()[..];
    let mut format_chars: Vec<u8> = Vec::with_capacity(fmt_src.len());
    let mut arg_elems: Punctuated<Expr, syn::Token![,]> = Punctuated::new();

    let int_variant_path: ExprPath = syn::parse_str("::evegfx::strfmt::Argument::Int").unwrap();
    let uint_variant_path: ExprPath = syn::parse_str("::evegfx::strfmt::Argument::UInt").unwrap();
    let char_variant_path: ExprPath = syn::parse_str("::evegfx::strfmt::Argument::Char").unwrap();
    let string_variant_path: ExprPath =
        syn::parse_str("::evegfx::strfmt::Argument::String").unwrap();

    let mut remain = fmt_src.clone();
    let mut next_arg = 0;
    let mut needs_fmt = false;
    while remain.len() > 0 {
        use parsers::Token::*;
        let (token, next) = parsers::next_token(remain);
        remain = next;
        match token {
            Literal(bytes) => {
                format_chars.extend(bytes);
            }
            Verb(bytes) => {
                needs_fmt = true;
                if next_arg >= args.len() {
                    let err = syn::Error::new(
                        fmt.span(),
                        format!("not enough arguments to populate {} verbs", next_arg + 1),
                    );
                    return err.into_compile_error().into();
                }
                let given_expr = args[next_arg].clone();
                next_arg += 1;

                format_chars.extend(bytes);
                // Our parser ensures that a format verb always includes at
                // least two bytes: the % and the verb letter. There might
                // be other stuff in between but we don't need to worry
                // about those because they'll be interpreted by EVE's
                // coprocessor, not by us. Our only goal here is to figure
                // out which enum variant to select for the argument.
                let mode = *bytes.last().unwrap();
                match mode {
                    b'd' | b'i' => {
                        let arg_expr = enum_variant_expr(int_variant_path.clone(), given_expr);
                        arg_elems.push(arg_expr);
                    }
                    b'u' | b'o' | b'x' | b'X' => {
                        let arg_expr = enum_variant_expr(uint_variant_path.clone(), given_expr);
                        arg_elems.push(arg_expr);
                    }
                    b'c' => {
                        let arg_expr = enum_variant_expr(char_variant_path.clone(), given_expr);
                        arg_elems.push(arg_expr);
                    }
                    b's' => {
                        let arg_expr = enum_variant_expr(string_variant_path.clone(), given_expr);
                        arg_elems.push(arg_expr);
                    }
                    // TODO: string pointers (%s) too
                    _ => {
                        // This is safe because our parser only allows ASCII
                        // letters as format strings.
                        use std::convert::TryInto;
                        let letter: char = mode.try_into().unwrap();

                        let err = syn::Error::new(
                            fmt.span(),
                            format!("unsupported format verb '%{}'", letter),
                        );
                        return err.into_compile_error().into();
                    }
                }
            }
            Percent(bytes) => {
                needs_fmt = true;
                format_chars.extend(bytes);
            }
            Null(_) => {
                // EVE's coprocessor considers a literal null to be a string
                // terminator, so we'll encode it instead as a format sequence
                // that inserts the null character in order to avoid producing
                // an invalid message.
                needs_fmt = true;
                format_chars.extend(b"%c");
                let arg_expr = enum_variant_expr(
                    uint_variant_path.clone(),
                    Expr::Lit(ExprLit {
                        attrs: Vec::new(),
                        lit: Lit::Int(syn::LitInt::new("0", fmt.span())),
                    }),
                );
                arg_elems.push(arg_expr);
            }
            Unterminated(_) => {
                let err = syn::Error::new(fmt.span(), "unterminated format sequence");
                return err.into_compile_error().into();
            }
            Invalid(_) => {
                let err = syn::Error::new(fmt.span(), "invalid format sequence");
                return err.into_compile_error().into();
            }
        };
    }

    if next_arg < args.len() {
        use syn::spanned::Spanned;
        let error_span = args[next_arg].span();
        let err = syn::Error::new(error_span, "too many arguments for format string");
        return err.into_compile_error().into();
    }

    // EVE expects format strings to be null-terminated.
    format_chars.push(0);

    let mut args: Punctuated<Expr, syn::Token![,]> = Punctuated::new();
    args.push(byte_string_expr(&format_chars, fmt.span()));
    if needs_fmt {
        args.push(array_slice_expr(arg_elems));
    }

    if needs_fmt {
        quote!(
            ::evegfx::strfmt::Message::new(#args)
        )
    } else {
        quote!(
            ::evegfx::strfmt::Message::new_literal(#args)
        )
    }
    .into()
}

fn enum_variant_expr(path: ExprPath, val: Expr) -> syn::Expr {
    let mut args: Punctuated<Expr, syn::Token![,]> = Punctuated::new();
    args.push(val);
    syn::Expr::Call(syn::ExprCall {
        attrs: Vec::new(),
        func: Box::new(Expr::Path(path)),
        args: args,
        paren_token: syn::token::Paren {
            span: Span::call_site(),
        },
    })
}

fn byte_string_expr(bytes: &[u8], span: proc_macro2::Span) -> syn::Expr {
    syn::Expr::Lit(syn::ExprLit {
        attrs: Vec::new(),
        lit: syn::LitByteStr::new(bytes, span).into(),
    })
}

fn array_slice_expr(elems: Punctuated<syn::Expr, syn::Token![,]>) -> syn::Expr {
    syn::Expr::Reference(syn::ExprReference {
        attrs: Vec::new(),
        and_token: syn::token::And {
            spans: [Span::call_site()],
        },
        mutability: None,
        expr: Box::new(syn::Expr::Array(syn::ExprArray {
            attrs: Vec::new(),
            bracket_token: syn::token::Bracket {
                span: Span::call_site(),
            },
            elems: elems,
        })),
        raw: std::default::Default::default(),
    })
}

struct EVEFormat {
    fmt: LitStr,
    args: Punctuated<Expr, syn::Token![,]>,
}

impl Parse for EVEFormat {
    fn parse(input: ParseStream) -> Result<Self> {
        let fmt: LitStr = input.parse()?;
        let args: Punctuated<Expr, syn::Token![,]> = if let Ok(_) = input.parse::<Token![,]>() {
            Punctuated::parse_terminated(input)?
        } else {
            Punctuated::new()
        };

        Ok(EVEFormat {
            fmt: fmt,
            args: args,
        })
    }
}
