use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use syncserver_common::Metrics;
use syncserver_settings::Settings;
use tokenserver_db_common::{DbError, DbPool, DbResult, MAX_GENERATION, params, results};

use crate::pool_from_settings;

#[tokio::test]
async fn test_update_generation() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            ..Default::default()
        })
        .await?
        .id;

    // Add a user
    let email = "test_user";
    let uid = db
        .post_user(params::PostUser {
            service_id,
            node_id,
            email: email.to_owned(),
            ..Default::default()
        })
        .await?
        .uid;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.generation, 0);
    assert_eq!(user.client_state, "");

    // Changing generation should leave other properties unchanged.
    db.put_user(params::PutUser {
        email: email.to_owned(),
        service_id,
        generation: 42,
        keys_changed_at: user.keys_changed_at,
    })
    .await?;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.node_id, node_id);
    assert_eq!(user.generation, 42);
    assert_eq!(user.client_state, "");

    // It's not possible to move the generation number backwards.
    db.put_user(params::PutUser {
        email: email.to_owned(),
        service_id,
        generation: 17,
        keys_changed_at: user.keys_changed_at,
    })
    .await?;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.node_id, node_id);
    assert_eq!(user.generation, 42);
    assert_eq!(user.client_state, "");

    Ok(())
}

#[tokio::test]
async fn test_update_keys_changed_at() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node".to_owned(),
            ..Default::default()
        })
        .await?
        .id;

    // Add a user
    let email = "test_user";
    let uid = db
        .post_user(params::PostUser {
            service_id,
            node_id,
            email: email.to_owned(),
            ..Default::default()
        })
        .await?
        .uid;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.keys_changed_at, None);
    assert_eq!(user.client_state, "");

    // Changing keys_changed_at should leave other properties unchanged.
    db.put_user(params::PutUser {
        email: email.to_owned(),
        service_id,
        generation: user.generation,
        keys_changed_at: Some(42),
    })
    .await?;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.node_id, node_id);
    assert_eq!(user.keys_changed_at, Some(42));
    assert_eq!(user.client_state, "");

    // It's not possible to move keys_changed_at backwards.
    db.put_user(params::PutUser {
        email: email.to_owned(),
        service_id,
        generation: user.generation,
        keys_changed_at: Some(17),
    })
    .await?;

    let user = db.get_user(params::GetUser { id: uid }).await?;

    assert_eq!(user.node_id, node_id);
    assert_eq!(user.keys_changed_at, Some(42));
    assert_eq!(user.client_state, "");

    Ok(())
}

#[tokio::test]
async fn replace_users() -> DbResult<()> {
    const MILLISECONDS_IN_A_MINUTE: i64 = 60 * 1000;
    const MILLISECONDS_IN_AN_HOUR: i64 = MILLISECONDS_IN_A_MINUTE * 60;

    let pool = db_pool().await?;
    let mut db = pool.get().await?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let an_hour_ago = now - MILLISECONDS_IN_AN_HOUR;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            ..Default::default()
        })
        .await?;

    // Add a user to be updated
    let email1 = "test_user_1";
    let uid1 = {
        // Set created_at to be an hour ago
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            })
            .await?
            .uid;

        db.set_user_created_at(params::SetUserCreatedAt {
            created_at: an_hour_ago,
            uid,
        })
        .await?;

        uid
    };

    // Add a user that has already been replaced
    let uid2 = {
        // Set created_at to be an hour ago
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            })
            .await?
            .uid;

        db.set_user_replaced_at(params::SetUserReplacedAt {
            replaced_at: an_hour_ago + MILLISECONDS_IN_A_MINUTE,
            uid,
        })
        .await?;

        db.set_user_created_at(params::SetUserCreatedAt {
            created_at: an_hour_ago,
            uid,
        })
        .await?;

        uid
    };

    // Add a user created too recently
    {
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            })
            .await?
            .uid;

        db.set_user_created_at(params::SetUserCreatedAt {
            created_at: now + MILLISECONDS_IN_AN_HOUR,
            uid,
        })
        .await?;
    }

    // Add a user with the wrong email address
    let email2 = "test_user_2";
    {
        // Set created_at to be an hour ago
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id: node_id.id,
                email: email2.to_owned(),
                ..Default::default()
            })
            .await?
            .uid;

        db.set_user_created_at(params::SetUserCreatedAt {
            created_at: an_hour_ago,
            uid,
        })
        .await?;
    }

    // Add a user with the wrong service
    {
        let uid = db
            .post_user(params::PostUser {
                service_id: service_id + 1,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            })
            .await?
            .uid;

        // Set created_at to be an hour ago
        db.set_user_created_at(params::SetUserCreatedAt {
            created_at: an_hour_ago,
            uid,
        })
        .await?;
    }

    // Perform the bulk update
    db.replace_users(params::ReplaceUsers {
        service_id,
        email: email1.to_owned(),
        replaced_at: now,
    })
    .await?;

    // Get all of the users
    let users = {
        let mut users1 = db
            .get_users(params::GetUsers {
                email: email1.to_owned(),
                service_id,
            })
            .await?;
        let mut users2 = db
            .get_users(params::GetUsers {
                email: email2.to_owned(),
                service_id,
            })
            .await?;
        users1.append(&mut users2);

        users1
    };

    let mut users_with_replaced_at_uids: Vec<i64> = users
        .iter()
        .filter(|user| user.replaced_at.is_some())
        .map(|user| user.uid)
        .collect();

    users_with_replaced_at_uids.sort_unstable();

    // The users with replaced_at timestamps should have the expected uids
    let mut expected_user_uids = vec![uid1, uid2];
    expected_user_uids.sort_unstable();
    assert_eq!(users_with_replaced_at_uids, expected_user_uids);

    Ok(())
}

#[tokio::test]
async fn post_user() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let post_node_params = params::PostNode {
        service_id,
        ..Default::default()
    };
    let node_id = db.post_node(post_node_params.clone()).await?.id;

    // Add a user
    let email1 = "test_user_1";
    let post_user_params1 = params::PostUser {
        service_id,
        email: email1.to_owned(),
        generation: 1,
        client_state: "aaaa".to_owned(),
        created_at: 2,
        node_id,
        keys_changed_at: Some(3),
    };
    let uid1 = db.post_user(post_user_params1.clone()).await?.uid;

    // Add another user
    let email2 = "test_user_2";
    let post_user_params2 = params::PostUser {
        service_id,
        node_id,
        email: email2.to_owned(),
        ..Default::default()
    };
    let uid2 = db.post_user(post_user_params2).await?.uid;

    // Ensure that two separate users were created
    assert_ne!(uid1, uid2);

    // Get a user
    let user = db.get_user(params::GetUser { id: uid1 }).await?;

    // Ensure the user has the expected values
    let expected_get_user = results::GetUser {
        service_id,
        email: email1.to_owned(),
        generation: 1,
        client_state: "aaaa".to_owned(),
        replaced_at: None,
        node_id,
        keys_changed_at: Some(3),
    };

    assert_eq!(user, expected_get_user);

    Ok(())
}

#[tokio::test]
async fn test_init_sync15_node() -> DbResult<()> {
    temp_env::async_with_vars(
        [
            (
                "SYNC_TOKENSERVER__INIT_NODE_URL",
                Some("https://testo.example.gg"),
            ),
            ("SYNC_TOKENSERVER__INIT_NODE_CAPACITY", Some("38383")),
        ],
        async {
            let pool = db_pool().await?;
            let mut db = pool.get().await?;

            let service_id = db
                .get_service_id(params::GetServiceId {
                    service: params::Sync15Node::SERVICE_NAME.to_owned(),
                })
                .await?
                .id;

            let node_id = db
                .get_node_id(params::GetNodeId {
                    service_id,
                    node: "https://testo.example.gg".to_owned(),
                })
                .await?
                .id;

            let node = db.get_node(params::GetNode { id: node_id }).await?;

            assert_eq!(node.node, "https://testo.example.gg");
            assert_eq!(node.capacity, 38383);
            assert_eq!(node.available, 1);
            assert_eq!(node.current_load, 0);

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_init_sync15_node_with_default_capacity() -> DbResult<()> {
    temp_env::async_with_vars(
        [
            (
                "SYNC_TOKENSERVER__INIT_NODE_URL",
                Some("https://testo.example.gg"),
            ),
            ("SYNC_TOKENSERVER__INIT_NODE_CAPACITY", None::<&str>),
        ],
        async {
            let pool = db_pool().await?;
            let mut db = pool.get().await?;

            let service_id = db
                .get_service_id(params::GetServiceId {
                    service: params::Sync15Node::SERVICE_NAME.to_owned(),
                })
                .await?
                .id;

            let node_id = db
                .get_node_id(params::GetNodeId {
                    service_id,
                    node: "https://testo.example.gg".to_owned(),
                })
                .await?
                .id;

            let node = db.get_node(params::GetNode { id: node_id }).await?;

            assert_eq!(node.node, "https://testo.example.gg");
            assert_eq!(node.capacity, 100000);

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn get_node_id() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id1 = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            ..Default::default()
        })
        .await?
        .id;

    // Add another node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node2".to_owned(),
        ..Default::default()
    })
    .await?;

    // Get the ID of the first node
    let id = db
        .get_node_id(params::GetNodeId {
            service_id,
            node: "https://node1".to_owned(),
        })
        .await?
        .id;

    // The ID should match that of the first node
    assert_eq!(node_id1, id);

    Ok(())
}

#[tokio::test]
async fn test_node_allocation() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?
        .id;

    // Allocating a user assigns it to the node
    let user = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;
    assert_eq!(user.node, "https://node1");

    // Getting the user from the database does not affect node assignment
    let user = db.get_user(params::GetUser { id: user.uid }).await?;
    assert_eq!(user.node_id, node_id);

    Ok(())
}

#[tokio::test]
async fn test_allocation_to_least_loaded_node() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add two nodes
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        ..Default::default()
    })
    .await?;

    db.post_node(params::PostNode {
        service_id,
        node: "https://node2".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        ..Default::default()
    })
    .await?;

    // Allocate two users
    let user1 = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test1@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let user2 = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test2@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // Because users are always assigned to the least-loaded node, the users should have been
    // assigned to different nodes
    assert_ne!(user1.node, user2.node);

    Ok(())
}

#[tokio::test]
async fn test_allocation_is_not_allowed_to_downed_nodes() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a downed node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        downed: 1,
        ..Default::default()
    })
    .await?;

    // User allocation fails because allocation is not allowed to downed nodes
    let result = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await;
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), "Unexpected error: unable to get a node");

    Ok(())
}

#[tokio::test]
async fn test_allocation_is_not_allowed_to_backoff_nodes() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a backoff node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        backoff: 1,
        ..Default::default()
    })
    .await?;

    // User allocation fails because allocation is not allowed to backoff nodes
    let result = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await;
    let error = result.unwrap_err();
    assert_eq!(error.to_string(), "Unexpected error: unable to get a node");

    Ok(())
}

#[tokio::test]
async fn test_node_reassignment_when_records_are_replaced() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        ..Default::default()
    })
    .await?;

    // Allocate a user
    let allocate_user_result = db
        .allocate_user(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;
    let user1 = db
        .get_user(params::GetUser {
            id: allocate_user_result.uid,
        })
        .await?;

    // Mark the user as replaced
    db.replace_user(params::ReplaceUser {
        uid: allocate_user_result.uid,
        service_id,
        replaced_at: 1234,
    })
    .await?;

    let user2 = db
        .get_or_create_user(params::GetOrCreateUser {
            email: "test@test.com".to_owned(),
            service_id,
            generation: 1235,
            client_state: "bbbb".to_owned(),
            keys_changed_at: Some(1235),
            capacity_release_rate: None,
        })
        .await?;

    // Calling get_or_create_user() results in the creation of a new user record, since the
    // previous record was marked as replaced
    assert_ne!(allocate_user_result.uid, user2.uid);

    // The account metadata should match that of the original user and *not* that in the
    // method parameters
    assert_eq!(user1.generation, user2.generation);
    assert_eq!(user1.keys_changed_at, user2.keys_changed_at);
    assert_eq!(user1.client_state, user2.client_state);

    Ok(())
}

#[tokio::test]
async fn test_node_reassignment_not_done_for_retired_users() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 0,
        capacity: 100,
        available: 100,
        ..Default::default()
    })
    .await?;

    // Add a retired user
    let user1 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: MAX_GENERATION,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let user2 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // Calling get_or_create_user() does not update the user's node
    assert_eq!(user1.uid, user2.uid);
    assert_eq!(user2.generation, MAX_GENERATION);
    assert_eq!(user1.client_state, user2.client_state);

    Ok(())
}

#[tokio::test]
async fn test_node_reassignment_and_removal() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add two nodes
    let node1_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?
        .id;

    let node2_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node2".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?
        .id;

    // Create four users. We should get two on each node.
    let user1 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test1@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let user2 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test2@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let user3 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test3@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let user4 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    let node1_count = [&user1, &user2, &user3, &user4]
        .iter()
        .filter(|user| user.node == "https://node1")
        .count();
    assert_eq!(node1_count, 2);
    let node2_count = [&user1, &user2, &user3, &user4]
        .iter()
        .filter(|user| user.node == "https://node2")
        .count();
    assert_eq!(node2_count, 2);

    // Clear the assignments on the first node.
    db.unassign_node(params::UnassignNode { node_id: node1_id })
        .await?;

    // The users previously on the first node should balance across both nodes,
    // giving 1 on the first node and 3 on the second node.
    let mut node1_count = 0;
    let mut node2_count = 0;

    for user in [&user1, &user2, &user3, &user4] {
        let new_user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                email: user.email.clone(),
                generation: user.generation,
                client_state: user.client_state.clone(),
                keys_changed_at: user.keys_changed_at,
                capacity_release_rate: None,
            })
            .await?;

        if new_user.node == "https://node1" {
            node1_count += 1;
        } else {
            assert_eq!(new_user.node, "https://node2");

            node2_count += 1;
        }
    }

    assert_eq!(node1_count, 1);
    assert_eq!(node2_count, 3);

    // Remove the second node. Everyone should end up on the first node.
    db.remove_node(params::RemoveNode { node_id: node2_id })
        .await?;

    // Every user should be on the first node now.
    for user in [&user1, &user2, &user3, &user4] {
        let new_user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                email: user.email.clone(),
                generation: user.generation,
                client_state: user.client_state.clone(),
                keys_changed_at: user.keys_changed_at,
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(new_user.node, "https://node1");
    }

    Ok(())
}

#[tokio::test]
async fn test_gradual_release_of_node_capacity() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add two nodes
    let node1_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 4,
            capacity: 8,
            available: 1,
            ..Default::default()
        })
        .await?
        .id;

    let node2_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node2".to_owned(),
            current_load: 4,
            capacity: 6,
            available: 1,
            ..Default::default()
        })
        .await?
        .id;

    // Two user creations should succeed without releasing capacity on either of the nodes.
    // The users should be assigned to different nodes.
    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test1@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node1");
    let node = db.get_node(params::GetNode { id: node1_id }).await?;
    assert_eq!(node.current_load, 5);
    assert_eq!(node.capacity, 8);
    assert_eq!(node.available, 0);

    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test2@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node2");
    let node = db.get_node(params::GetNode { id: node2_id }).await?;
    assert_eq!(node.current_load, 5);
    assert_eq!(node.capacity, 6);
    assert_eq!(node.available, 0);

    // The next allocation attempt will release 10% more capacity, which is one more slot for
    // each node.
    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test3@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node1");
    let node = db.get_node(params::GetNode { id: node1_id }).await?;
    assert_eq!(node.current_load, 6);
    assert_eq!(node.capacity, 8);
    assert_eq!(node.available, 0);

    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node2");
    let node = db.get_node(params::GetNode { id: node2_id }).await?;
    assert_eq!(node.current_load, 6);
    assert_eq!(node.capacity, 6);
    assert_eq!(node.available, 0);

    // Now that node2 is full, further allocations will go to node1.
    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test5@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node1");
    let node = db.get_node(params::GetNode { id: node1_id }).await?;
    assert_eq!(node.current_load, 7);
    assert_eq!(node.capacity, 8);
    assert_eq!(node.available, 0);

    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test6@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    assert_eq!(user.node, "https://node1");
    let node = db.get_node(params::GetNode { id: node1_id }).await?;
    assert_eq!(node.current_load, 8);
    assert_eq!(node.capacity, 8);
    assert_eq!(node.available, 0);

    // Once the capacity is reached, further user allocations will result in an error.
    let result = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test7@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "Unexpected error: unable to get a node"
    );

    Ok(())
}

#[tokio::test]
async fn test_correct_created_at_used_during_node_reassignment() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 4,
            capacity: 8,
            available: 1,
            ..Default::default()
        })
        .await?
        .id;

    // Create a user
    let user1 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // Clear the user's node
    db.unassign_node(params::UnassignNode { node_id }).await?;

    // Sleep very briefly to ensure the timestamp created during node reassignment is greater
    // than the timestamp created during user creation
    thread::sleep(Duration::from_millis(5));

    // Get the user, prompting the user's reassignment to the same node
    let user2 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // The user's timestamp should be updated since a new user record was created.
    assert!(user2.created_at > user1.created_at);

    Ok(())
}

#[tokio::test]
async fn test_correct_created_at_used_during_user_retrieval() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    db.post_node(params::PostNode {
        service_id,
        node: "https://node1".to_owned(),
        current_load: 4,
        capacity: 8,
        available: 1,
        ..Default::default()
    })
    .await?;

    // Create a user
    let user1 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // Sleep very briefly to ensure that any timestamp that might be created below is greater
    // than the timestamp created during user creation
    thread::sleep(Duration::from_millis(5));

    // Get the user
    let user2 = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            generation: 1234,
            email: "test4@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })
        .await?;

    // The user's timestamp should be equal to the one generated when the user was created
    assert_eq!(user1.created_at, user2.created_at);

    Ok(())
}

#[tokio::test]
async fn test_latest_created_at() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node
    let node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?
        .id;

    let email = "test_user";
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Add a user marked as replaced
    let post_user = params::PostUser {
        service_id,
        email: email.to_owned(),
        node_id,
        client_state: "aaaa".to_owned(),
        generation: 1234,
        keys_changed_at: Some(1234),
        created_at: now,
    };
    let uid1 = db.post_user(post_user.clone()).await?.uid;
    db.replace_user(params::ReplaceUser {
        uid: uid1,
        service_id,
        replaced_at: now,
    })
    .await?;

    // User's latest record w/ a new client_state, otherwise identical to the
    // replaced record (even created_at)
    let uid2 = db
        .post_user(params::PostUser {
            client_state: "bbbb".to_owned(),
            ..post_user
        })
        .await?
        .uid;
    assert_ne!(uid1, uid2);

    // Should return the latest record even with the identical created_at
    let user = db
        .get_or_create_user(params::GetOrCreateUser {
            service_id,
            email: email.to_owned(),
            ..Default::default()
        })
        .await?;
    assert_eq!(user.uid, uid2);
    assert_eq!(user.client_state, "bbbb");

    Ok(())
}

#[tokio::test]
async fn test_get_spanner_node() -> DbResult<()> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;

    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    // Add a node with capacity and available set to 0
    let spanner_node_id = db
        .post_node(params::PostNode {
            service_id,
            node: "https://spanner_node".to_owned(),
            current_load: 1000,
            capacity: 0,
            available: 0,
            ..Default::default()
        })
        .await?
        .id;

    // Add another node with available capacity
    db.post_node(params::PostNode {
        service_id,
        node: "https://another_node".to_owned(),
        current_load: 0,
        capacity: 1000,
        available: 1000,
        ..Default::default()
    })
    .await?;

    // Ensure the node with available capacity is selected if the Spanner node ID is not
    // cached
    assert_ne!(
        db.get_best_node(params::GetBestNode {
            service_id,
            capacity_release_rate: None,
        })
        .await?
        .id,
        spanner_node_id
    );

    // Ensure the Spanner node is selected if the Spanner node ID is cached
    db.set_spanner_node_id(Some(spanner_node_id as i32));

    assert_eq!(
        db.get_best_node(params::GetBestNode {
            service_id,
            capacity_release_rate: None,
        })
        .await?
        .id,
        spanner_node_id
    );

    Ok(())
}

#[tokio::test]
async fn heartbeat() -> Result<(), DbError> {
    let pool = db_pool().await?;
    let mut db = pool.get().await?;
    assert!(db.check().await?);
    Ok(())
}

async fn db_pool() -> DbResult<Box<dyn DbPool>> {
    let _ = env_logger::try_init();

    let mut settings = Settings::test_settings();
    settings.tokenserver.run_migrations = true;
    let use_test_transactions = true;

    let mut pool = pool_from_settings(
        &settings.tokenserver,
        &Metrics::noop(),
        use_test_transactions,
    )?;
    pool.init().await?;

    Ok(pool)
}
