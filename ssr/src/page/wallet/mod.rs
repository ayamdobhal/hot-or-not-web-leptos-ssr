pub mod tokens;
pub mod transactions;
pub mod txn;
use crate::{
    component::share_popup::ShareButtonWithFallbackPopup, state::canisters::unauth_canisters,
};
use candid::Principal;
use leptos::*;
use leptos_router::Params;
use leptos_router::{use_params, Redirect};
use tokens::TokenList;

use crate::{
    component::{canisters_prov::AuthCansProvider, connect::ConnectLogin},
    state::{auth::account_connected_reader, canisters::authenticated_canisters},
    try_or_redirect_opt,
    utils::profile::ProfileDetails,
};

#[component]
fn ProfileGreeter(details: ProfileDetails, is_own_account: bool) -> impl IntoView {
    // let (is_connected, _) = account_connected_reader();
    let share_link = {
        let principal = details.principal();
        format!("/wallet/{}", principal)
    };
    let message = format!(
        "Hey! Check out my YRAL profile 👇 {}. I just minted my own token—come see and create yours! 🚀 #YRAL #TokenMinter",
        share_link
    );

    view! {
        <div class="flex flex-col">
            {is_own_account
                .then(|| {
                    view! { <span class="text-white/50 text-md">Welcome!</span> }
                })} <div class="flex flex-row gap-2">
                <span class="text-lg text-white md:text-xl truncate">
                    // TEMP: Workaround for hydration bug until leptos 0.7
                    // class=("md:w-5/12", move || !is_connected())
                    {details.display_name_or_fallback()}

                </span>
                <ShareButtonWithFallbackPopup share_link message />
            </div>
        </div>
        <div class="justify-self-end w-16 rounded-full aspect-square overflow-clip">
            <img class="object-cover w-full h-full" src=details.profile_pic_or_random() />
        </div>
    }
}

#[component]
fn FallbackGreeter() -> impl IntoView {
    view! {
        <div class="flex flex-col">
            <span class="text-white/50 text-md">Welcome!</span>
            <div class="py-2 w-3/4 rounded-full animate-pulse bg-white/40"></div>
        </div>
        <div class="justify-self-end w-16 rounded-full animate-pulse aspect-square overflow-clip bg-white/40"></div>
    }
}

#[component]
fn BalanceFallback() -> impl IntoView {
    view! { <div class="py-3 mt-1 w-1/4 rounded-full animate-pulse bg-white/30"></div> }
}

#[derive(Params, PartialEq)]
struct WalletParams {
    id: String,
}
#[component]
pub fn Wallet() -> impl IntoView {
    let params = use_params::<WalletParams>();
    let param_principal = move || {
        params.with(|p| {
            let WalletParams { id, .. } = p.as_ref().ok()?;
            Principal::from_text(id).ok()
        })
    };

    view! {
        {move || {
            match param_principal() {
                Some(principal) => view! { <WalletImpl principal /> },
                None => {
                    view! {
                        <AuthCansProvider let:cans>
                            {move || {
                                view! {
                                    <Redirect path=format!("/wallet/{}", cans.user_principal()) />
                                }
                            }}
                        </AuthCansProvider>
                    }
                }
            }
        }}
    }
}
#[component]
pub fn WalletImpl(principal: Principal) -> impl IntoView {
    let (is_connected, _) = account_connected_reader();

    let auth_cans = authenticated_canisters();
    let balance_fetch = create_resource(
        move || principal,
        move |principal| async move {
            let canisters = unauth_canisters();
            let Some(user_canister) = canisters
                .get_individual_canister_by_user_principal(principal)
                .await?
            else {
                return Err(ServerFnError::new("Failed to get user canister"));
            };
            let user = canisters.individual_user(user_canister).await;

            let bal = user.get_utility_token_balance().await?;
            Ok::<_, ServerFnError>(bal.to_string())
        },
    );

    let profile_info_res = auth_cans.derive(
        move || principal,
        move |cans_wire, principal| async move {
            let cans_wire = cans_wire?;
            let canisters = cans_wire.clone().canisters()?;

            let Some(user_canister) = canisters
                .get_individual_canister_by_user_principal(principal)
                .await?
            else {
                return Err(ServerFnError::new("Failed to get user canister"));
            };
            let user = canisters.individual_user(user_canister).await;
            let user_details = user.get_profile_details().await?;
            Ok::<ProfileDetails, ServerFnError>(user_details.into())
        },
    );

    let is_own_account = auth_cans.derive(
        move || principal,
        move |cans_wire, principal| async move {
            let cans_wire = cans_wire?;
            let canisters = cans_wire.clone().canisters()?;
            Ok::<_, ServerFnError>(canisters.user_principal() == principal)
        },
    );

    let canister_id = create_resource(
        move || principal,
        move |principal| async move {
            let canisters = unauth_canisters();
            let Some(user_canister) = canisters
                .get_individual_canister_by_user_principal(principal)
                .await?
            else {
                return Err(ServerFnError::new("Failed to get user canister"));
            };
            Ok((user_canister, principal))
        },
    );
    view! {
        <div>
            <div class="flex flex-col gap-4 px-4 pt-4 pb-12 bg-black min-h-dvh">
                <div class="grid grid-cols-2 grid-rows-1 items-center w-full">
                    <Suspense>
                        {move || {
                            let profile_details = try_or_redirect_opt!(profile_info_res()?);
                            let is_own_account = try_or_redirect_opt!(is_own_account()?);
                            Some(
                                view! { <ProfileGreeter details=profile_details is_own_account /> },
                            )
                        }}
                    </Suspense>
                </div>
                <div class="flex flex-col items-center mt-6 w-full text-white">
                    <Suspense>
                        {move || {
                            let is_own_account = try_or_redirect_opt!(is_own_account() ?);
                            let balance = try_or_redirect_opt!(balance_fetch() ?);
                            Some(
                                view! {
                                    <span class="uppercase lg:text-lg text-md">
                                        {if is_own_account {
                                            "Your Coyns Balance"
                                        } else {
                                            "Coyns Balance"
                                        }}
                                    </span>
                                    <div class="text-xl lg:text-2xl">{balance}</div>
                                },
                            )
                        }}
                    </Suspense>
                </div>
                <Suspense>
                    {move || {
                        let is_own_account = try_or_redirect_opt!(is_own_account() ?);
                        Some(
                            view! {
                                <Show when=move || !is_connected() && is_own_account>
                                    <div class="flex flex-col items-center py-5 w-full">
                                        <div class="flex flex-row items-center w-9/12 md:w-5/12">
                                            <ConnectLogin
                                                login_text="Login to claim your COYNs"
                                                cta_location="wallet"
                                            />
                                        </div>
                                    </div>
                                </Show>
                            },
                        )
                    }}
                </Suspense>
                <div class="flex flex-col gap-2 w-full">
                    <Suspense>
                        {move || {
                            let is_own_account = try_or_redirect_opt!(is_own_account()?);
                            Some(
                                view! {
                                    <div class="flex flex-row justify-between items-end w-full">
                                        <span class="text-sm text-white md:text-md">
                                            {if is_own_account { "My Tokens" } else { "Tokens" }}
                                        </span>
                                    </div>
                                },
                            )
                        }}
                    </Suspense>
                    <div class="flex flex-col gap-2 items-center">
                        <Suspense>
                            {move || {
                                let canister_id = try_or_redirect_opt!(canister_id() ?);
                                Some(
                                    view! {
                                        <TokenList user_principal=canister_id.1 user_canister=canister_id.0 />
                                    },
                                )
                            }}
                        </Suspense>
                    </div>
                </div>
            </div>
        </div>
    }
}
