use std::sync::Arc;

use futures::StreamExt;
use sc_network::NetworkService;
use sc_network::{NetworkDHTProvider, NetworkEventStream};
use sc_service::TaskManager;
use sp_runtime::traits::Block as BlockT;

pub fn notification_worker<'a, TBl>(
	network: Arc<NetworkService<TBl, <TBl as BlockT>::Hash>>,
	task_manager: &'a mut TaskManager,
) where
	TBl: BlockT,
{
	let key = "apple".as_bytes().to_vec();
	let key2 = "apple".as_bytes().to_vec();

	network.put_value(key.into(), "this_is_me".as_bytes().to_vec());
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
