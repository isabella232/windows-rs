use crate::*;
use squote::{quote, TokenStream};

pub fn gen_async(name: &TypeName, interfaces: &[RequiredInterface]) -> (TokenStream, TokenStream) {
    let kind = async_kind(name);
    if kind != AsyncKind::None {
        return gen_async_kind(kind, name, name);
    }

    for interface in interfaces {
        let kind = async_kind(&interface.name);

        if kind != AsyncKind::None {
            return gen_async_kind(kind, &interface.name, name);
        }
    }

    (TokenStream::new(), TokenStream::new())
}

#[derive(PartialEq)]
enum AsyncKind {
    None,
    Action,
    ActionWithProgress,
    Operation,
    OperationWithProgress,
}

fn async_kind(name: &TypeName) -> AsyncKind {
    if name.namespace != "Windows.Foundation" {
        return AsyncKind::None;
    }

    match name.name.as_ref() {
        "IAsyncAction" => AsyncKind::Action,
        "IAsyncActionWithProgress`1" => AsyncKind::ActionWithProgress,
        "IAsyncOperation`1" => AsyncKind::Operation,
        "IAsyncOperationWithProgress`2" => AsyncKind::OperationWithProgress,
        _ => AsyncKind::None,
    }
}

fn gen_async_kind(
    kind: AsyncKind,
    name: &TypeName,
    self_name: &TypeName,
) -> (TokenStream, TokenStream) {
    let return_type = match kind {
        AsyncKind::Operation | AsyncKind::OperationWithProgress => name.generics[0].gen(),
        _ => quote! { () },
    };

    let handler = match kind {
        AsyncKind::Action => quote! { AsyncActionCompletedHandler },
        AsyncKind::ActionWithProgress => quote! { AsyncActionWithProgressCompletedHandler },
        AsyncKind::Operation => quote! { AsyncOperationCompletedHandler },
        AsyncKind::OperationWithProgress => quote! { AsyncOperationWithProgressCompletedHandler },
        _ => panic!("Unexpected AsyncKind"),
    };

    let constraints = self_name.gen_constraint();
    let name = self_name.gen();

    (
        quote! {
            pub fn get(&self) -> ::windows::Result<#return_type> {
                if self.status()? == ::windows::foundation::AsyncStatus::Started {
                    let (waiter, signaler) = ::windows::Waiter::new();
                    self.set_completed(::windows::foundation:: #handler::new(move |_sender, _args| {
                        // Safe because the waiter will only be dropped after being signaled.
                        unsafe { signaler.signal(); }
                        Ok(())
                    }))?;
                }
                self.get_results()
            }
        },
        quote! {
            impl<#constraints> ::std::future::Future for #name {
                type Output = ::windows::Result<#return_type>;

                fn poll(self: ::std::pin::Pin<&mut Self>, context: &mut ::std::task::Context) -> ::std::task::Poll<Self::Output> {
                    if self.status()? == ::windows::foundation::AsyncStatus::Started {
                        let waker = context.waker().clone();

                        let _ = self.set_completed(::windows::foundation:: #handler::new(move |_sender, _args| {
                            waker.wake_by_ref();
                            Ok(())
                        }));

                        ::std::task::Poll::Pending
                    } else {
                        ::std::task::Poll::Ready(self.get_results())
                    }
                }
            }
        },
    )
}
