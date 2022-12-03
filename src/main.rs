use clap::ArgMatches;
use cosmos_sdk_proto::cosmos::base::query::v1beta1::PageRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::{
    QueryValidatorDelegationsRequest, QueryValidatorsRequest,
};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use tonic::transport::{Channel, Endpoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap::Command::new("cosmos-hodler-snapshot")
        .version("0.1.0")
        .about("Snapshot token stakers on Cosmos SDK chains")
        .subcommand_required(true)
        .author("wesl-ee")
        .arg(
            clap::arg!(--"grpc" <URI>)
                .required(true)
                .value_parser(clap::value_parser!(Endpoint)),
        )
        .subcommand(clap::command!("native-stakers"))
        .get_matches();

    let endpoint = matches.get_one::<Endpoint>("grpc").unwrap();
    let channel = match tokio::time::timeout(
        tokio::time::Duration::from_secs(3),
        endpoint.connect(),
    )
    .await
    {
        Ok(channel) => channel,
        Err(_) => panic!("gRPC timed out"),
    }?;

    match matches.subcommand() {
        Some(("native-stakers", matches)) => {
            native_stakers(matches, channel).await?
        }
        _ => unreachable!(),
    };

    Ok(())
}

async fn native_stakers(
    _matches: &ArgMatches,
    channel: Channel,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut csv = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("juno_stakers.csv")
        .unwrap();

    let mut validators: Vec<String> = vec![];
    let mut pagination = None;

    let mut staking_query_client = QueryClient::<Channel>::new(channel);
    loop {
        let val_response = staking_query_client
            .validators(QueryValidatorsRequest {
                pagination,
                status: "BOND_STATUS_BONDED".to_string(),
            })
            .await?
            .into_inner();

        validators.append(
            &mut val_response
                .validators
                .iter()
                .map(|v| v.operator_address.clone())
                .collect::<Vec<String>>(),
        );

        pagination = match val_response.pagination {
            Some(p) => {
                if p.next_key.is_empty() {
                    break;
                }

                Some(PageRequest {
                    key: p.next_key,
                    offset: 0,
                    limit: 100,
                    count_total: false,
                    reverse: false,
                })
            }
            None => break,
        };
    }

    let mut delegators_map: HashMap<String, u128> = HashMap::new();
    for (i, v) in validators.iter().enumerate() {
        println!(
            "[{:.1}%] Processing delegations to validator {}",
            (i * 100) as f64 / validators.len() as f64,
            v
        );

        pagination = None;
        loop {
            let del_response = staking_query_client
                .validator_delegations(QueryValidatorDelegationsRequest {
                    pagination,
                    validator_addr: v.to_string(),
                })
                .await?
                .into_inner();

            for resp in del_response.delegation_responses {
                if let Some(delegation) = resp.delegation {
                    let amount =
                        resp.balance.unwrap().amount.parse::<u128>().unwrap();
                    let new_stake = match delegators_map
                        .get(&delegation.delegator_address)
                    {
                        Some(stake) => stake + amount,
                        None => amount,
                    };

                    delegators_map
                        .insert(delegation.delegator_address, new_stake);
                }
            }

            pagination = match del_response.pagination {
                Some(p) => {
                    if p.next_key.is_empty() {
                        break;
                    }

                    Some(PageRequest {
                        key: p.next_key,
                        offset: 0,
                        limit: 100,
                        count_total: false,
                        reverse: false,
                    })
                }
                None => break,
            };
        }

        println!("{} delegators indexed", delegators_map.keys().len());
    }

    for (d, amt) in delegators_map {
        if amt > 0 {
            writeln!(csv, "{},{}", d, amt).unwrap();
        }
    }

    Ok(())
}
