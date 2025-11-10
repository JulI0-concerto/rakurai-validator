use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    pub stake_pool_ids: Vec<String>,
    pub validators_config: HashMap<String, ValidatorConfig>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub stakers: Option<StakersConfig>,
    pub validator_reward_split: Option<ValidatorRewardSplit>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StakersConfig {
    pub excluded_stake_pubkeys: Option<Vec<String>>,
    pub custom_commissions: Option<HashMap<String, CommissionConfig>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CommissionConfig {
    pub commission_bps: u32,
    #[serde(default)]
    pub referral_claimant_pubkey: Option<String>,
    #[serde(default)]
    pub referral_claimant_commission_bps: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValidatorRewardSplit {
    pub claimant_pubkey: String,
    pub claimant_commission_bps: u32,
}

pub const MAX_COMMISSION_BPS: u32 = 10_000;
impl DistributionConfig {
    pub fn validate_and_normalize(&mut self, min_commission_bps: u32) {
        for (validator, vcfg) in self.validators_config.iter_mut() {
            // === Handle custom commissions ===
            if let Some(stakers) = &mut vcfg.stakers {
                if let Some(custom_commissions) = &mut stakers.custom_commissions {
                    let mut to_remove = vec![];

                    for (staker, cfg) in custom_commissions.iter_mut() {
                        let before = cfg.commission_bps;
                        info!("staker {:?} cfg {:?}", staker, cfg);

                        // clamp commission to [min_commission_bps, MAX_COMMISSION_BPS]
                        if cfg.commission_bps > min_commission_bps {
                            cfg.commission_bps = min_commission_bps;
                            warn!(
                                "Validator {} staker {} commission increased: {} → {}",
                                validator, staker, before, cfg.commission_bps
                            );
                        } else if cfg.commission_bps > MAX_COMMISSION_BPS {
                            cfg.commission_bps = MAX_COMMISSION_BPS;
                            warn!(
                                "Validator {} staker {} commission capped: {} → {}",
                                validator, staker, before, cfg.commission_bps
                            );
                        }

                        // === Clean up referral ===
                        if let Some(bps) = cfg.referral_claimant_commission_bps {
                            if bps == 0 {
                                cfg.referral_claimant_commission_bps = None;
                                cfg.referral_claimant_pubkey = None;
                                warn!(
                                    "Validator {} staker {} referral removed (0 commission)",
                                    validator, staker
                                );
                            }
                        }

                        // prune configs only if it's *entirely default-like* (no referral, commission == min)
                        if cfg.commission_bps == min_commission_bps
                            && cfg.referral_claimant_pubkey.is_none()
                            && cfg.referral_claimant_commission_bps.is_none()
                        {
                            to_remove.push(staker.clone());
                            warn!(
                                "Validator {} staker {} config removed (equal to min, no referral)",
                                validator, staker
                            );
                        }
                    }

                    // actually remove redundant configs
                    for key in to_remove {
                        custom_commissions.remove(&key);
                    }

                    // drop map if empty
                    if custom_commissions.is_empty() {
                        stakers.custom_commissions = None;
                    }
                }
            }

            // === Handle validator reward split ===
            if let Some(split) = &mut vcfg.validator_reward_split {
                let before = split.claimant_commission_bps;

                if split.claimant_commission_bps > MAX_COMMISSION_BPS {
                    split.claimant_commission_bps = MAX_COMMISSION_BPS;
                    warn!(
                        "Validator {} claimant commission capped: {} → {}",
                        validator, before, split.claimant_commission_bps
                    );
                }

                if split.claimant_commission_bps == 0 {
                    warn!(
                        "Validator {} reward split removed (0 commission)",
                        validator
                    );
                    vcfg.validator_reward_split = None;
                }
            }
        }
    }
}
