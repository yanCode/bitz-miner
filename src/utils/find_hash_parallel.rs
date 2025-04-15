use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

use anyhow::Result;
use colored::Colorize;
use drillx::{Hash, Solution, equix};
use solana_rpc_client::spinner;
use tokio::sync::mpsc::UnboundedSender;

use super::format_duration;

pub async fn find_hash_parallel(
    challenge: [u8; 32],
    cutoff_time: u64,
    cores: u64,
    min_difficulty: u32,
    nonce_indices: &[u64],
    pool_channel: Option<UnboundedSender<Solution>>,
) -> Result<Solution> {
    // Dispatch job to each thread
    let progress_bar = Arc::new(spinner::new_progress_bar());
    let global_best_difficulty = Arc::new(RwLock::new(0u32));

    progress_bar.set_message("Collecting...");
    let core_ids = core_affinity::get_core_ids().expect("Failed to fetch core count");
    let core_ids = core_ids.into_iter().filter(|id| id.id < (cores as usize));
    let handles: Vec<_> = core_ids
        .map(|i| {
            let global_best_difficulty = Arc::clone(&global_best_difficulty);
            std::thread::spawn({
                let progress_bar = progress_bar.clone();
                let nonce = nonce_indices[i.id];
                let mut memory = equix::SolverMemory::new();
                let pool_channel = pool_channel.clone();
                move || {
                    // Pin to core
                    let _ = core_affinity::set_for_current(i);

                    // Start hashing
                    let timer = Instant::now();
                    let mut nonce = nonce;
                    let mut best_nonce = nonce;
                    let mut best_difficulty = 0;
                    let mut best_hash = Hash::default();
                    loop {
                        // Get hashes
                        let hxs = drillx::hashes_with_memory(
                            &mut memory,
                            &challenge,
                            &nonce.to_le_bytes(),
                        );

                        // Look for best difficulty score in all hashes
                        for hx in hxs {
                            let difficulty = hx.difficulty();
                            if difficulty.gt(&best_difficulty) {
                                best_nonce = nonce;
                                best_difficulty = difficulty;
                                best_hash = hx;
                                if best_difficulty.gt(&*global_best_difficulty.read().unwrap()) {
                                    // Update best global difficulty
                                    *global_best_difficulty.write().unwrap() = best_difficulty;

                                    // Continuously upload best solution to pool
                                    if difficulty.ge(&min_difficulty) {
                                        if let Some(ref ch) = pool_channel {
                                            let digest = best_hash.d;
                                            let nonce = nonce.to_le_bytes();
                                            let solution = Solution {
                                                d: digest,
                                                n: nonce,
                                            };
                                            if let Err(err) = ch.send(solution) {
                                                println!("{} {:?}", "ERROR".bold().red(), err);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Exit if time has elapsed
                        if nonce % 100 == 0 {
                            let global_best_difficulty = *global_best_difficulty.read().unwrap();
                            if timer.elapsed().as_secs().ge(&cutoff_time) {
                                if i.id == 0 {
                                    progress_bar.set_message(format!(
                                        "Collecting...\n  Best score: {}",
                                        global_best_difficulty,
                                    ));
                                }
                                if global_best_difficulty.ge(&min_difficulty) {
                                    // Collect until min difficulty has been met
                                    break;
                                }
                            } else if i.id == 0 {
                                progress_bar.set_message(format!(
                                    "Collecting...\n  Best score: {}\n  Time remaining: {}",
                                    global_best_difficulty,
                                    format_duration(
                                        cutoff_time.saturating_sub(timer.elapsed().as_secs())
                                            as u32
                                    ),
                                ));
                            }
                        }

                        // Increment nonce
                        nonce += 1;
                    }

                    // Return the best nonce
                    (best_nonce, best_difficulty, best_hash)
                }
            })
        })
        .collect();

    // Join handles and return best nonce
    let mut best_nonce: u64 = 0;
    let mut best_difficulty = 0;
    let mut best_hash = Hash::default();
    for h in handles {
        if let Ok((nonce, difficulty, hash)) = h.join() {
            if difficulty > best_difficulty {
                best_difficulty = difficulty;
                best_nonce = nonce;
                best_hash = hash;
            }
        }
    }

    Ok(Solution::new(best_hash.d, best_nonce.to_le_bytes()))
}
