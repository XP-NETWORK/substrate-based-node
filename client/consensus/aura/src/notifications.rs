use std::sync::Arc;
use std::thread;
use std::time::Duration;

use futures::StreamExt;
use sc_network::multiaddr::multihash::Code;
use sc_network::{NetworkService, KademliaKey};
use sc_network::{NetworkDHTProvider, NetworkEventStream};
use sc_service::TaskManager;
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_keystore::Keystore;
use sp_keystore::testing::MemoryKeystore;
use sp_runtime::traits::Block as BlockT;
use sc_network::multiaddr::multihash::MultihashDigest;
pub fn notification_worker<'a, TBl>(
	network: Arc<NetworkService<TBl, <TBl as BlockT>::Hash>>,
	task_manager: &'a mut TaskManager,
) where
	TBl: BlockT,
{
	let key2 = "apple".as_bytes().to_vec();

	let network_to_put_value = Arc::clone(&network);

	task_manager
		.spawn_handle()
		.spawn_blocking("AuraNotificationWorker2", None, async move {
			println!("AURA PUTTING VALUES");
			let remote_key_store = MemoryKeystore::new();

			let key: AuthorityId = remote_key_store
				.sr25519_generate_new(key_types::AUTHORITY_DISCOVERY, None)
				.unwrap()
				.into();

			let key = KademliaKey::new(&Code::Sha2_256.digest(key.as_ref()).digest());
			// let key = "apple".as_bytes().to_vec();

			loop {
				network_to_put_value
					.put_value(key.clone(), "this_is_me".as_bytes().to_vec());
				println!("AURA - VALUE PUT");
				thread::sleep(Duration::from_secs(10));
			}
		});

	let mut stream = network.event_stream("aura_notification_worker");

	task_manager
		.spawn_handle()
		.spawn_blocking("AuraNotificationWorker", None, async move {
			println!("AURA I am inside SPAWN");
			while let Some(s) = stream.next().await {
				println!("AURA I am some stream obj {:#?}", s);
			}
		});

	let _ = network.get_value(&key2.into());
}
