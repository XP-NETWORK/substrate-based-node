use std::sync::Arc;
use std::thread;
use std::time::Duration;

use distributed_frost::{ FrostInfo, create_participant, keygen::{ Participant, Coefficients } };
use futures::StreamExt;
use sc_network::{ NetworkService, KademliaKey };
use sc_network::{ NetworkDHTProvider, NetworkEventStream };
use sc_service::TaskManager;
use sp_application_crypto::key_types;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_keystore::Keystore;
use sp_keystore::testing::MemoryKeystore;
use sp_runtime::traits::Block as BlockT;

use serde::{ Serialize, Deserialize };
use bincode;

use generic_array::{ GenericArray, typenum::B0, typenum::B1, typenum::UTerm };
/// Type Alias for public_bytes type
pub type PublicBytes = GenericArray<
	u8,
	sha3::digest::typenum::UInt<
		sha3::digest::typenum::UInt<
			sha3::digest::typenum::UInt<
				sha3::digest::typenum::UInt<
					sha3::digest::typenum::UInt<sha3::digest::typenum::UInt<UTerm, B1>, B0>,
					B0
				>,
				B0
			>,
			B0
		>,
		B1
	>
>;
#[derive(Serialize, Deserialize, Debug)]
struct ValidatorData<'a> {
	id: u8,
	message: &'a str,
}

impl<'a> ValidatorData<'a> {
	fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
		bincode::serialize(&self)
	}

	fn from_bytes(bytes: &'a [u8]) -> Result<Self, bincode::Error> {
		bincode::deserialize(bytes)
	}
}

const FROST_THRESHOLD: u8 = 7;
const FROST_TOTAL_VALIDATORS: u8 = 11;

pub fn notification_worker<'a, TBl>(
	network: Arc<NetworkService<TBl, <TBl as BlockT>::Hash>>,
	task_manager: &'a mut TaskManager
)
	where TBl: BlockT
{
	let dht_key: KademliaKey = "apple".as_bytes().to_vec().into();
	let dht_key_for_put = dht_key.clone(); // Clone the key for the put operation
	let dht_key_for_get = dht_key.clone(); // Clone the key for the get operation

	let front_info = FrostInfo {
		totalvalue: FROST_TOTAL_VALIDATORS,
		thresholdvalue: FROST_THRESHOLD,
	};

	let (party, public_bytes, partycoeffs): (
		Participant,
		PublicBytes,
		Coefficients,
	) = create_participant(front_info, 1);
	println!("public key, {:?}", party.public_key().unwrap());
	// Task to periodically put the value into the DHT.
	let network_for_put = Arc::clone(&network); // Clone for the put operation
	task_manager.spawn_handle().spawn_blocking("AuraPutValueWorker", None, async move {
		// let remote_key_store = MemoryKeystore::new();
		// let _authority_key: AuthorityId = remote_key_store
		// 	.sr25519_generate_new(key_types::AUTHORITY_DISCOVERY, None)
		// 	.unwrap()
		// 	.into();

		thread::sleep(Duration::from_secs(30));
		loop {
			network_for_put.put_value(dht_key_for_put.clone(), public_bytes.to_vec());
			println!("AURA - VALUE PUT");
			thread::sleep(Duration::from_secs(10));
		}
	});

	// Task to periodically request the value from the DHT and print it.
	let network_for_get = Arc::clone(&network); // Clone for the get operation
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
							println!("Found value in DHT: {:?}", public_bytes.to_vec() == value);
						}
					}
				}
				sc_network::Event::Dht(sc_network::DhtEvent::ValueNotFound(queried_key)) if
					&queried_key == &dht_key
				=> {
					println!("Value not found in DHT for key: {:?}", queried_key);
				}
				_ => {} // Handle other events if necessary.
			}
		}
	});
}
