#![cfg(test)]

use common_lib::types::v0::{
    store::nexus::NexusSpec,
    transport::{
        CreateVolume, DestroyShutdownTargets, GetSpecs, Nexus, PublishVolume, RepublishVolume,
        VolumeShareProtocol,
    },
};
use deployer_cluster::ClusterBuilder;
use grpc::operations::{registry::traits::RegistryOperations, volume::traits::VolumeOperations};
use std::time::Duration;

#[tokio::test]
async fn lazy_delete_shutdown_targets() {
    const POOL_SIZE_BYTES: u64 = 128 * 1024 * 1024;
    let cluster = ClusterBuilder::builder()
        .with_rest(true)
        .with_agents(vec!["core"])
        .with_io_engines(2)
        .with_tmpfs_pool(POOL_SIZE_BYTES)
        .with_cache_period("1s")
        .with_reconcile_period(Duration::from_secs(1), Duration::from_secs(1))
        .build()
        .await
        .unwrap();

    let vol_cli = cluster.grpc_client().volume();
    let volume = vol_cli
        .create(
            &CreateVolume {
                uuid: "1e3cf927-80c2-47a8-adf0-95c486bdd7b7".try_into().unwrap(),
                size: 5242880,
                replicas: 2,
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    let volume = vol_cli
        .publish(
            &PublishVolume {
                uuid: volume.uuid().clone(),
                share: None,
                target_node: Some(cluster.node(0)),
            },
            None,
        )
        .await
        .unwrap();
    let first_target = volume.state().target.unwrap();

    cluster
        .composer()
        .kill(cluster.node(0).as_str())
        .await
        .unwrap();

    vol_cli
        .republish(
            &RepublishVolume {
                uuid: volume.uuid().clone(),
                target_node: Some(cluster.node(1)),
                share: VolumeShareProtocol::Nvmf,
            },
            None,
        )
        .await
        .unwrap();

    cluster.restart_core().await;
    // Wait for core service to restart.
    cluster
        .node_service_liveness(None)
        .await
        .expect("Should have restarted by now");

    vol_cli
        .destroy_shutdown_target(
            &DestroyShutdownTargets {
                uuid: volume.uuid().clone(),
            },
            None,
        )
        .await
        .expect("Should destroy old target even though the node is offline!");

    let nx_cli = cluster.grpc_client().registry();

    let target = find_target(&nx_cli, &first_target).await;
    assert!(target.unwrap().spec_status.deleting());

    cluster
        .composer()
        .restart(cluster.node(0).as_str())
        .await
        .unwrap();

    wait_till_target_deleted(&nx_cli, &first_target).await;
}

async fn find_target(client: &impl RegistryOperations, target: &Nexus) -> Option<NexusSpec> {
    let response = client.get_specs(&GetSpecs {}, None).await.unwrap().nexuses;
    response.into_iter().find(|n| n.uuid == target.uuid)
}

/// Wait for the unpublished volume to have the specified replica count
pub(crate) async fn wait_till_target_deleted(client: &impl RegistryOperations, target: &Nexus) {
    let timeout = Duration::from_secs(11);
    let start = std::time::Instant::now();
    loop {
        let target = find_target(client, target).await;
        if target.is_none() {
            return;
        }

        if std::time::Instant::now() > (start + timeout) {
            panic!("Timeout waiting for the target to be deleted");
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}