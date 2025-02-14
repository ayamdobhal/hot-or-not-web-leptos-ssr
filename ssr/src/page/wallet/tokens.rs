use std::str::FromStr;

use candid::{Nat, Principal};
use ic_agent::AgentError;

use crate::page::token::RootType;
use crate::page::wallet::ShareButtonWithFallbackPopup;
use crate::utils::token::{get_ck_metadata, TokenBalanceOrClaiming};
use crate::{
    component::infinite_scroller::{CursoredDataProvider, InfiniteScroller, KeyedData, PageEntry},
    state::canisters::{unauth_canisters, Canisters},
    utils::{
        profile::propic_from_principal,
        token::{token_metadata_by_root, TokenBalance, TokenMetadata},
    },
};
use futures::stream::{self, StreamExt};
use leptos::*;
use yral_canisters_client::individual_user_template::Result14;
use yral_canisters_client::sns_ledger::{Account, SnsLedger};
#[derive(Clone)]
pub struct TokenRootList {
    pub canisters: Canisters<false>,
    pub user_canister: Principal,
    pub user_principal: Principal,
}

impl KeyedData for Principal {
    type Key = Principal;

    fn key(&self) -> Self::Key {
        *self
    }
}

impl KeyedData for RootType {
    type Key = RootType;

    fn key(&self) -> Self::Key {
        self.clone()
    }
}
async fn get_balance<'a>(user_principal: Principal, ledger: &SnsLedger<'a>) -> Option<Nat> {
    ledger
        .icrc_1_balance_of(Account {
            owner: user_principal,
            subaccount: None,
        })
        .await
        .ok()
}

impl CursoredDataProvider for TokenRootList {
    type Data = RootType;
    type Error = AgentError;

    async fn get_by_cursor(
        &self,
        start: usize,
        end: usize,
    ) -> Result<PageEntry<Self::Data>, Self::Error> {
        let user = self.canisters.individual_user(self.user_canister).await;
        let tokens = user
            .get_token_roots_of_this_user_with_pagination_cursor(start as u64, end as u64)
            .await?;
        let mut tokens: Vec<RootType> = match tokens {
            Result14::Ok(v) => v
                .into_iter()
                .map(|t| RootType::from_str(&t.to_text()).unwrap())
                .collect(),
            Result14::Err(_) => vec![],
        };
        let list_end = tokens.len() < (end - start);
        if start == 0 {
            let rep = stream::iter([
                RootType::from_str("btc").unwrap(),
                RootType::from_str("usdc").unwrap(),
            ])
            .filter_map(|root_type| async move {
                let cans = unauth_canisters();

                match root_type {
                    RootType::BTC { ledger, .. } => {
                        let ledger = cans.sns_ledger(ledger).await;
                        let bal = get_balance(self.user_principal, &ledger).await?;

                        if bal != 0u64 {
                            Some(root_type)
                        } else {
                            None
                        }
                    }
                    RootType::USDC { ledger, .. } => {
                        let ledger = cans.sns_ledger(ledger).await;
                        let bal = get_balance(self.user_principal, &ledger).await?;

                        if bal != 0u64 {
                            Some(root_type)
                        } else {
                            None
                        }
                    }
                    _ => Some(root_type),
                }
            })
            .collect::<Vec<_>>()
            .await;
            tokens.splice(0..0, rep);
        }
        Ok(PageEntry {
            data: tokens,
            end: list_end,
        })
    }
}

async fn token_metadata_or_fallback(
    cans: Canisters<false>,
    user_principal: Principal,
    token_root: Principal,
) -> TokenMetadata {
    let metadata = token_metadata_by_root(&cans, Some(user_principal), token_root)
        .await
        .ok()
        .flatten();
    metadata.unwrap_or_else(|| TokenMetadata {
        logo_b64: propic_from_principal(token_root),
        name: "<ERROR>".to_string(),
        description: "Unknown".to_string(),
        symbol: "??".to_string(),
        balance: Some(TokenBalanceOrClaiming::claiming()),
        fees: TokenBalance::new_cdao(0u32.into()),
        root: Some(Principal::anonymous()),
        ledger: Principal::anonymous(),
        index: Principal::anonymous(),
        decimals: 8,
    })
}

#[component]
pub fn TokenViewFallback() -> impl IntoView {
    view! {
        <div class="w-full items-center h-16 rounded-xl border-2 border-neutral-700 bg-white/15 animate-pulse"></div>
    }
}

#[component]
pub fn TokenView(
    user_principal: Principal,
    token_root: RootType,
    #[prop(optional)] _ref: NodeRef<html::A>,
) -> impl IntoView {
    let info = create_resource(
        move || (token_root.clone(), user_principal),
        move |(token_root, user_principal)| async move {
            let cans = unauth_canisters();

            match token_root {
                RootType::BTC { ledger, index } => {
                    get_ck_metadata(cans, Some(user_principal), ledger, index)
                        .await
                        .unwrap()
                        .unwrap()
                }
                RootType::USDC { ledger, index } => {
                    get_ck_metadata(cans, Some(user_principal), ledger, index)
                        .await
                        .unwrap()
                        .unwrap()
                }
                RootType::Other(root) => {
                    token_metadata_or_fallback(cans.clone(), user_principal, root).await
                }
            }
        },
    );

    view! {
        <Suspense fallback=TokenViewFallback>
            {move || {
                info.map(|info| {
                    view! {
                        <TokenTile
                            user_principal
                            token_meta_data=info.clone()
                        />
                    }
                })
            }}

        </Suspense>
    }
}

fn generate_share_link_from_metadata(
    token_meta_data: &TokenMetadata,
    user_principal: Principal,
) -> String {
    format!(
        "/token/info/{}/{user_principal}?airdrop_amt=100",
        token_meta_data
            .root
            .map(|r| r.to_text())
            .unwrap_or(token_meta_data.name.to_lowercase())
    )
}

#[component]
pub fn TokenTile(user_principal: Principal, token_meta_data: TokenMetadata) -> impl IntoView {
    let share_link = generate_share_link_from_metadata(&token_meta_data, user_principal);
    let share_link_s = store_value(share_link);
    let share_message = format!(
        "Hey! Check out the token: {} I created on YRAL 👇 {}. I just minted my own token—come see and create yours! 🚀 #YRAL #TokenMinter",
        token_meta_data.symbol,
        share_link_s(),
    );
    let share_message_s = store_value(share_message);
    let info = token_meta_data;
    view! {
        <div class="flex  w-full items-center h-16 rounded-xl border-2 border-neutral-700 bg-white/15 gap-1">
            <a
                href=share_link_s()
                // _ref=_ref
                class="flex flex-1  p-y-4"
            >
                <div class="flex flex-2 items-center space-x-2 px-2">
                    <img class="w-12 h-12 rounded-full" src=info.logo_b64.clone() />
                    <span class="text-white text-xs truncate">{info.name.clone()}</span>
                </div>
                <div class="flex flex-1 flex-col">
                    <span class="flex flex-1  items-center justify-end text-xs text-white">
                        // remove the unwrap if global token listing but its a list of token so it can safely be unwrapped
                        {info.balance.unwrap().humanize_float_truncate_to_dp(2)}
                    </span>
                    <span class="flex flex-1  items-center justify-end text-xs text-white truncate">
                        {info.symbol.clone()}
                    </span>
                </div>

            </a>
            <div>
                <ShareButtonWithFallbackPopup
                    share_link=share_link_s()
                    message=share_message_s()
                    style="w-12 h-12".into()
                />
            </div>

        </div>
    }
}

#[component]
pub fn TokenList(user_principal: Principal, user_canister: Principal) -> impl IntoView {
    let canisters = unauth_canisters();

    let provider: TokenRootList = TokenRootList {
        canisters,
        user_canister,
        user_principal,
    };

    view! {
        <div class="flex flex-col w-full gap-2 items-center">
            <InfiniteScroller
                provider
                fetch_count=10
                children=move |token_root, _ref| {
                    view! { <TokenView user_principal token_root=token_root _ref=_ref.unwrap_or_default() /> }
                }
            />

        </div>
    }
}
