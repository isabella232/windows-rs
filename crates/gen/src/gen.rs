use super::*;
use std::iter::FromIterator;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Gen {
    Absolute,
    Relative(&'static str),
}

impl Gen {
    // TODO: remove this now that we have fixed the build macro - this isn't needed anymore
    pub(crate) fn windows(&self) -> TokenStream {
        match self {
            Self::Absolute => quote! { ::windows:: },
            Self::Relative(_) => quote! { ::windows:: },
        }
    }

    pub fn namespace(&self, namespace: &str) -> TokenStream {
        match self {
            Self::Absolute => {
                let mut tokens = TokenStream::new();

                for namespace in namespace.split('.') {
                    let namespace = to_ident(&to_snake(namespace));

                    tokens.combine(&quote! { #namespace:: });
                }

                tokens
            }
            Self::Relative(relative) => {
                if namespace == *relative {
                    return TokenStream::new();
                }

                let mut tokens = Vec::new();
                let mut relative = relative.split('.').peekable();
                let mut namespace = namespace.split('.').peekable();

                while relative.peek() == namespace.peek() {
                    if relative.next().is_none() {
                        break;
                    }
                    namespace.next();
                }

                let count = relative.count();

                if count > 0 {
                    tokens.resize(tokens.len() + count, quote! { super:: });
                }

                tokens.extend(namespace.map(|namespace| {
                    let namespace = to_ident(&to_snake(namespace));
                    quote! { #namespace:: }
                }));

                TokenStream::from_iter(tokens)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows() {
        assert_eq!(
            ElementType::String.gen_name(Gen::Absolute).as_str(),
            "windows :: HString"
        );

        assert_eq!(
            ElementType::String.gen_name(Gen::Relative("test")).as_str(),
            ":: windows :: HString"
        );
    }

    #[test]
    fn test_namespace() {
        let reader = TypeReader::get();
        let t = reader.resolve_type("Windows.Foundation", "IStringable");

        assert_eq!(
            t.gen_name(Gen::Absolute).as_str(),
            "windows :: foundation :: IStringable"
        );

        assert_eq!(
            t.gen_name(Gen::Relative("Windows")).as_str(),
            "foundation :: IStringable"
        );

        assert_eq!(
            t.gen_name(Gen::Relative("Windows.Foundation")).as_str(),
            "IStringable"
        );

        assert_eq!(
            t.gen_name(Gen::Relative("Windows.Foundation.Collections"))
                .as_str(),
            "super :: IStringable"
        );
    }
}
