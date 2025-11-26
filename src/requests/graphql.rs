use serde_json::json;

pub fn build_recent_deployments_query(
    id: &str,
    type_: &str,
    public_key: &str,
    address: &str,
    limit: u32,
) -> serde_json::Value {
    let query = r#"
        query GetRecentDeployments($publicKey: String!, $address: String!, $limit: Int!) {
            deployments(
                where: {
                    _or: [
                        { deployer: { _eq: $publicKey } }
                        { transfers: { to_address: { _eq: $address } } }
                    ]
                }
                order_by: { block_number: desc }
                limit: $limit
            ) {
                deploy_id
                block_number
                created_at
                status
                block {
                    finalization_status
                }
                transfers {
                    id
                    from_address
                    to_address
                    amount_dust
                }
            }
        }
    "#;

    json!({
        "id": id,
        "type": type_,
        "payload": {
            "query": query,
            "variables": {
                "publicKey": public_key,
                "address": address,
                "limit": limit
            }
        }
    })
}

pub fn build_new_deployments_subscription(
    id: &str,
    type_: &str,
    public_key: &str,
    address: &str,
    start_time: &str,
) -> serde_json::Value {
    let query = r#"
        subscription WatchNewDeployments($publicKey: String!, $address: String!, $startTime: timestamp!) {
            deployments(
                where: {
                    _and: [
                        { created_at: { _gt: $startTime } }
                        {
                            _or: [
                                { deployer: { _eq: $publicKey } }
                                { transfers: { to_address: { _eq: $address } } }
                            ]
                        }
                    ]
                }
                order_by: { block_number: asc }
            ) {
                deploy_id
                block_number
                created_at
                status
                block {
                    finalization_status
                }
                transfers {
                    id
                    from_address
                    to_address
                    amount_dust
                }
            }
        }
    "#;

    json!({
        "id": id,
        "type": type_,
        "payload": {
            "query": query,
            "variables": {
                "publicKey": public_key,
                "address": address,
                "startTime": start_time
            }
        }
    })
}
