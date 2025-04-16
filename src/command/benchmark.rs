use crate::{Miner, args::BenchmarkArgs, constants::BENCHMARK_TEST_DURATION};
use anyhow::Result;
use core_affinity::{get_core_ids, set_for_current};
use drillx::equix;
use solana_rpc_client::spinner;
use std::{sync::Arc, thread, time::Instant};

impl Miner {
    pub async fn benchmark(&self, args: BenchmarkArgs) -> Result<()> {
        // Check num threads
        let cores = self.parse_cores(args.cores);
        self.check_num_cores(cores)?;

        // Dispatch job to each thread
        let challenge = [0; 32];
        let progress_bar = Arc::new(spinner::new_progress_bar());
        progress_bar.set_message(format!(
            "Benchmarking. This will take {} sec...",
            BENCHMARK_TEST_DURATION
        ));
        let core_ids = get_core_ids().expect("Failed to fetch core count");
        let handles: Vec<_> = core_ids
            .into_iter()
            .map(|i| {
                thread::spawn({
                    move || {
                        let timer = Instant::now();
                        let first_nonce =
                            u64::MAX.saturating_div(cores).saturating_mul(i.id as u64);
                        let mut nonce = first_nonce;
                        let mut memory = equix::SolverMemory::new();
                        loop {
                            // Return if core should not be used
                            if (i.id as u64).ge(&cores) {
                                return 0;
                            }

                            // Pin to core, it might not be supported on modern macOS/Apple Silicon
                            let _ = set_for_current(i);

                            // Create hash
                            if let Ok(_) = drillx::hash_with_memory(
                                &mut memory,
                                &challenge,
                                &nonce.to_le_bytes(),
                            ) {
                                // Increment nonce
                                nonce += 1;
                            }

                            // Exit if time has elapsed
                            if (timer.elapsed().as_secs() as i64).ge(&BENCHMARK_TEST_DURATION) {
                                break;
                            }
                        }

                        // Return hash count
                        nonce - first_nonce
                    }
                })
            })
            .collect();

        // Join handles and return best nonce
        let mut total_nonces = 0;
        for h in handles {
            if let Ok(count) = h.join() {
                total_nonces += count;
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Hashpower: {} H/sec",
            total_nonces.saturating_div(BENCHMARK_TEST_DURATION as u64),
        ));
        Ok(())
    }
}
