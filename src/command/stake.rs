use crate::{
    Miner,
    args::{StakeArgs, StakeCommand},
};
use anyhow::Result;

impl Miner {
    pub async fn stake(&self, args: StakeArgs) -> Result<()> {
        if let Some(subcommand) = args.command {
            match subcommand {
                StakeCommand::Claim(_args) => todo!(),
                StakeCommand::Deposit(_args) => todo!(),
                StakeCommand::Withdraw(_args) => todo!(),
                StakeCommand::Accounts(_args) => todo!(),
            }
        } else {
            match args.mint {
                Some(_mint) => todo!(), //get authority
                None => todo!(),        //list
            }
        }
    }
}

