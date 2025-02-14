use candid::Principal;
use futures::future::join_all;
use leptos::*;

use crate::state::canisters::Canisters;
use crate::utils::token::token_metadata_by_root;

pub const SUPPORTED_NON_YRAL_TOKENS_ROOT: &[&str] = &["67bll-riaaa-aaaaq-aaauq-cai"];

pub async fn eligible_non_yral_supported_tokens(
    cans: Canisters<true>,
    user_principal: Principal,
) -> Result<Vec<Principal>, ServerFnError> {
    let tasks: Vec<_> = SUPPORTED_NON_YRAL_TOKENS_ROOT
        .iter()
        .map(|&token_root| {
            let cans = cans.clone();

            async move {
                let token_root = Principal::from_text(token_root).ok()?;
                let metadata_res = token_metadata_by_root(&cans, Some(user_principal), token_root)
                    .await
                    .ok()?;
                if let Some(metadata) = metadata_res {
                    if metadata
                        .balance?
                        .map_balance_ref(|b| b.e8s > 0u64)
                        .unwrap_or_default()
                    {
                        return Some(token_root);
                    } else {
                        return None;
                    }
                }

                None
            }
        })
        .collect();

    let task_results: Vec<Option<Principal>> = join_all(tasks).await;

    let eligible_token_root: Vec<Principal> = task_results.into_iter().flatten().collect();

    Ok(eligible_token_root)
}
