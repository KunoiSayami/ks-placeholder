use std::io::{Read, Write};

use proc_macro2::TokenStream;
use quote::TokenStreamExt;

const DEFAULT_HEADER_LENGTH: usize = 50;
const DEFAULT_HEADER: &[u8; DEFAULT_HEADER_LENGTH] =
    b"// PLACEHOLDER FILE DEFAULT HEADER, DO NOT CHANGE\n";

fn read_and_write<P: AsRef<std::path::Path> + std::fmt::Display>(
    span: proc_macro2::Span,
    filename: &P,
    input: TokenStream,
) -> syn::Result<()> {
    #[cfg(debug_assertions)]
    eprintln!("[{}] Checking {filename}", env!("CARGO_PKG_NAME"));

    if filename.as_ref().exists() {
        let mut file = std::fs::File::open(filename)
            .map_err(|e| syn::Error::new(span, format!("Open {filename} error: {e:?}")))?;

        let mut buffer: [u8; DEFAULT_HEADER_LENGTH] = [0; DEFAULT_HEADER_LENGTH];

        let read_size = file
            .read(&mut buffer)
            .map_err(|e| syn::Error::new(span, format!("Read {filename} header error: {e:?}")))?;

        if read_size != DEFAULT_HEADER_LENGTH || !buffer.eq(DEFAULT_HEADER) {
            #[cfg(debug_assertions)]
            eprintln!(
                "[{}] {filename} file not managed, keep unchanged",
                env!("CARGO_PKG_NAME")
            );
            return Ok(());
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)
        .map_err(|e| syn::Error::new(span, format!("Open {filename} error: {e:?}")))?;

    file.write_all(DEFAULT_HEADER)
        .map_err(|e| syn::Error::new(span, format!("Write {filename} header error: {e:?}")))?;

    file.write_all(input.to_string().as_bytes())
        .map_err(|e| syn::Error::new(span, format!("Write {filename} body error: {e:?}")))?;

    Ok(())
}

fn process(input: TokenStream) -> syn::Result<TokenStream> {
    let mut v: Vec<_> = input.clone().into_iter().collect();

    if v.len() < 3 {
        return Err(syn::Error::new_spanned(input, "Usage: (macro_name)! (<$filename:literal> <punctuate> <$content:others...>)\nSample: (macro_name)! { \"src/foo.rs\"; pub mod Foo {} }"));
    }

    let file_name = v.get(0).unwrap();

    let span = file_name.span();
    let file_name = match file_name {
        proc_macro2::TokenTree::Literal(f) => {
            let s = f.to_string();
            if !(s.starts_with('"') && (s.ends_with('"'))) {
                return Err(syn::Error::new_spanned(
                    file_name,
                    "Must be string in index 0",
                ));
            }
            s[1..s.len() - 1].to_string()
        }
        _ => {
            return Err(syn::Error::new_spanned(
                file_name,
                "Must be literal in index 0",
            ))
        }
    };

    let punctuate = v.get(1).unwrap();

    match punctuate {
        proc_macro2::TokenTree::Punct(_) => {}
        _ => {
            return Err(syn::Error::new_spanned(
                punctuate,
                "Must be punctuate in index 1",
            ))
        }
    }
    v.drain(..2);

    let ret = v.iter().fold(TokenStream::new(), |mut acc, x| {
        acc.append(x.clone());
        acc
    });

    read_and_write(span, &file_name, ret)?;

    println!("cargo::rerun-if-changed={}", file_name);

    Ok(Default::default())
}

#[proc_macro]
pub fn placeholder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    process(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
