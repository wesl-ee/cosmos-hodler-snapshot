use clap::ArgMatches;
use cosmos_sdk_proto::cosmos::base::query::v1beta1::PageRequest;
use cosmos_sdk_proto::cosmos::staking::v1beta1::query_client::QueryClient;
use cosmos_sdk_proto::cosmos::staking::v1beta1::{
    QueryValidatorDelegationsRequest, QueryValidatorDelegationsResponse,
    QueryValidatorsRequest, QueryValidatorsResponse,
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
        .subcommand(
            clap::command!("native-stakers").arg(
                clap::arg!(--"output" <PATH>)
                    .value_parser(clap::value_parser!(std::path::PathBuf)),
            ),
        )
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
        /* Unreachable because a command is required */
        _ => unreachable!(),
    };

    Ok(())
}

/**
 * Snapshot stakers of the native token (x/staking)
 */
async fn native_stakers(
    matches: &ArgMatches,
    channel: Channel,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = matches
        .get_one::<std::path::PathBuf>("output")
        .unwrap_or(&std::path::PathBuf::from("juno_stakers.csv"))
        .clone();

    /* Open the csv here so we don't do all the processing just to find out it
     * can't be opened created or written to */
    let mut csv = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .unwrap();

    let mut staking_query_client = QueryClient::<Channel>::new(channel);

    let validators =
        validators_with_status(&mut staking_query_client, "".to_string())
            .await?;

    let mut delegators_map: HashMap<String, u128> = HashMap::new();
    for (i, v) in validators.iter().enumerate() {
        println!(
            "[{:.1}%] Processing delegations to validator {}",
            (i * 100) as f64 / (validators.len() - 1) as f64,
            v
        );

        let mut pagination = None;
        loop {
            /* gRPC query */
            let del_response = staking_query_client
                .validator_delegations(QueryValidatorDelegationsRequest {
                    pagination,
                    validator_addr: v.to_string(),
                })
                .await?
                .into_inner();

            /* Set up the next queriable page, if any */
            pagination = next_page(&del_response);

            /* Break down the response into individual delegators and index */
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

            /* This was the final page */
            if pagination.is_none() {
                break;
            }
        }

        println!("{} delegators indexed", delegators_map.keys().len());
    }

    for (d, amt) in delegators_map {
        if amt > 0 {
            /* The thinking man's csv library */
            writeln!(csv, "{},{}", d, amt).unwrap();
        }
    }

    Ok(())
}

/**
 * Query the gRPC for all non-jailed validators of given status
 * + BOND_STATUS_BONDED
 * + BOND_STATUS_UNBONDING
 * + BOND_STATUS_UNBONDED
 *
 * Or "" for all validators
 */
async fn validators_with_status(
    client: &mut QueryClient<Channel>,
    status: String,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut validators = vec![];
    let mut pagination = None;
    loop {
        let response = client
            .validators(QueryValidatorsRequest {
                pagination,
                status: status.clone(),
            })
            .await?
            .into_inner();

        validators.append(
            &mut response
                .validators
                .iter()
                .filter(|v| !v.jailed)
                .map(|v| v.operator_address.clone())
                .collect::<Vec<String>>(),
        );
        pagination = match next_page(&response) {
            Some(p) => Some(p),
            None => return Ok(validators),
        };
    }
}

/**
 * Construct the next PageRequest to send given the previous response
 * to a paginated query
 */
fn next_page<T: PaginatedResponse>(resp: &T) -> Option<PageRequest> {
    match resp.next_key() {
        Some(key) => {
            if key.is_empty() {
                None
            } else {
                Some(PageRequest {
                    /* Last page's returned key */
                    key: key.to_vec(),
                    /* 100 per page */
                    limit: 100,
                    /* Don't specify offset - mutually exclusive w/ "key" */
                    offset: 0,
                    reverse: false,
                    count_total: false,
                })
            }
        }
        None => None,
    }
}

/**
 * A response from the SDK that returns a `next_key` for fetching the next
 * page of data
 */
pub trait PaginatedResponse {
    fn next_key(&self) -> Option<Vec<u8>>;
}

impl PaginatedResponse for QueryValidatorDelegationsResponse {
    fn next_key(&self) -> Option<Vec<u8>> {
        self.pagination
            .clone() // NG
            .map(|p| p.next_key)
    }
}

impl PaginatedResponse for QueryValidatorsResponse {
    fn next_key(&self) -> Option<Vec<u8>> {
        self.pagination
            .clone() // NG
            .map(|p| p.next_key)
    }
}
