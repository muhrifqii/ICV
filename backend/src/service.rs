use std::sync::Arc;

use crate::entities::{User, UserRepository};
use context::IcvCtx;

pub mod errors {
    use thiserror::Error;

    #[derive(Error, Debug, PartialEq, Eq, Clone)]
    pub enum UserError {
        #[error(r#"User identity {identity} cannot be found."#)]
        IdentityNotFound { identity: String },
    }
}

pub mod context {
    #[cfg(all(test, not(rust_analyzer)))]
    use crate::utils::mock_ic0::caller;
    use candid::Principal;
    #[cfg(any(not(test), rust_analyzer))]
    use ic_cdk::caller;

    use super::errors::UserError;
    use crate::entities::{IdentityProvider, User, USER_REPOSITORY};

    #[derive(Clone, Debug)]
    pub struct IcvCtx {
        caller: Principal,
        user: Option<User>,
    }

    impl Default for IcvCtx {
        fn default() -> Self {
            Self {
                caller: Principal::anonymous(),
                user: None,
            }
        }
    }

    impl IcvCtx {
        pub fn get() -> Self {
            let caller = caller();

            Self {
                caller,
                user: USER_REPOSITORY.get_user(caller),
            }
        }

        pub fn user(&self) -> Result<User, UserError> {
            self.user.clone().ok_or(UserError::IdentityNotFound {
                identity: self.caller.to_string(),
            })
        }

        pub fn caller(&self) -> Principal {
            self.caller
        }
    }

    #[cfg(test)]
    mod tests {
        use super::IcvCtx;
        use crate::{mock_ic0, IndexedRepository, Repository, User, USER_REPOSITORY};
        use candid::Principal;

        #[test]
        fn init_ctx_should_have_anon_caller() {
            let ctx = IcvCtx::default();
            assert_eq!(ctx.caller(), Principal::anonymous());
            assert!(ctx.user.is_none());
        }

        #[test]
        fn get_ctx_should_query_user() {
            let id_str =
                String::from("bx5fm-umlzd-vxxln-dg7bd-xb6xi-zs26l-lslll-zgnje-bhguv-ov47m-zqe");
            let identity = Principal::from_text(id_str.clone()).unwrap();
            USER_REPOSITORY
                .insert(User {
                    id: 1,
                    fullname: "fulan".to_string(),
                    identity: identity.clone(),
                    resume: String::new(),
                })
                .unwrap();
            mock_ic0::set_caller(id_str);
            let ctx = IcvCtx::get();
            assert_eq!(identity, ctx.caller);
            assert_eq!("fulan", ctx.user().unwrap().fullname);

            USER_REPOSITORY.clear_indexes();
            mock_ic0::reset_caller();
        }
    }
}

#[derive(Debug, Default)]
pub struct UserService {
    user_repository: Arc<UserRepository>,
}

impl UserService {
    /// Registers a new user for the caller identity.
    pub fn register(&self, ctx: &IcvCtx) {}
}
