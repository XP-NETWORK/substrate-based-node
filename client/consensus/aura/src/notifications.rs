use std::sync::Arc;
use std::thread;
use std::time::Duration;

use futures::StreamExt;
use sc_network::multiaddr::multihash::Code;
use sc_network::{ NetworkService, KademliaKey };
use sc_network::{ NetworkDHTProvider, NetworkEventStream };
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
)
where
    TBl: BlockT,
{
    let dht_key: KademliaKey = "apple".as_bytes().to_vec().into();
    let dht_key_for_put = dht_key.clone();  // Clone the key for the put operation
    let dht_key_for_get = dht_key.clone();  // Clone the key for the get operation

    // Task to periodically put the value into the DHT.
    let network_for_put = Arc::clone(&network);  // Clone for the put operation
    task_manager.spawn_handle().spawn_blocking("AuraPutValueWorker", None, async move {
        let remote_key_store = MemoryKeystore::new();
        let _authority_key: AuthorityId = remote_key_store
            .sr25519_generate_new(key_types::AUTHORITY_DISCOVERY, None)
            .unwrap()
            .into();

        thread::sleep(Duration::from_secs(30));

        loop {
            network_for_put.put_value(dht_key_for_put.clone(), "this_is_me".as_bytes().to_vec());
            println!("AURA - VALUE PUT");
            thread::sleep(Duration::from_secs(10));
        }
    });

    // Task to periodically request the value from the DHT and print it.
    let network_for_get = Arc::clone(&network);  // Clone for the get operation
    task_manager.spawn_handle().spawn_blocking("AuraGetValueWorker", None, async move {
        loop {
            network_for_get.get_value(&dht_key_for_get);
            println!("AURA - VALUE GET");
            thread::sleep(Duration::from_secs(10));
        }
    });

    // Task to listen to network events and handle them.
    let mut stream = network.event_stream("aura_notification_worker");
    task_manager.spawn_handle().spawn_blocking("AuraNotificationWorker", None, async move {
        println!("AURA I am inside SPAWN");
        while let Some(event) = stream.next().await {
            match event {
                sc_network::Event::Dht(sc_network::DhtEvent::ValueFound(records)) => {
                    for (found_key, value) in records {
                        if &found_key == &dht_key {
                            println!("Found value in DHT: {:?}", std::str::from_utf8(&value));
                        }
                    }
                },
                sc_network::Event::Dht(sc_network::DhtEvent::ValueNotFound(queried_key)) if &queried_key == &dht_key => {
                    println!("Value not found in DHT for key: {:?}", queried_key);
                },
                _ => {}  // Handle other events if necessary.
            }
        }
    });
}

