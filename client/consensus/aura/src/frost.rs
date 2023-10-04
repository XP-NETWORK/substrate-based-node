/// Module contains functions related to frost
pub mod frost {
	use std::{sync::Arc, thread, time::Duration};

	use futures::StreamExt;
	use sc_service::{Configuration, TaskManager};
	use subxt::{OnlineClient, PolkadotConfig};

	use std::{fmt::Debug, hash::Hash};

	use codec::{Decode, Encode};
	use sc_client_api::{backend::AuxStore, BlockOf};
	use sp_api::ProvideRuntimeApi;
	use sp_application_crypto::AppPublic;
	use sp_blockchain::HeaderBackend;
	use sp_core::crypto::Pair;
	use sp_runtime::traits::{Block as BlockT, Member};

	pub use crate::standalone::{find_pre_digest, slot_duration};
	pub use sc_consensus_slots::SlotProportion;
	pub use sp_consensus::SyncOracle;
	pub use sp_consensus_aura::{
		digests::CompatibleDigestItem,
		inherents::{InherentDataProvider, InherentType as AuraInherent, INHERENT_IDENTIFIER},
		AuraApi, ConsensusLog, SlotDuration, AURA_ENGINE_ID,
	};
	/// The error enum for Frost Worker
	#[derive(Debug)]
	pub enum FrostWorkerError {
		/// Failure in submitting tx to the substrate runtime
		FailedToSubmitTransaction,
		/// Submitted tx to substrate runtime failed
		SubstrateTransactionFailed,
		///
		FailedToSubscribeToBlocks,
		///
		FailedToCreateSubstrateClient,
	}

	#[subxt::subxt(runtime_metadata_path = "./src/metadata.scale")]
	pub mod bridge {}

	type AuthorityId<P> = <P as Pair>::Public;

	/// Start a seperate worker for continously check for any cluster without group key
	/// If theres a compelete cluster i.e a cluster has 11 validators and group key is not generated for i
	/// then start the group key generation.
	pub fn start_frost_worker<'a, P, B, C>(
		config: &'a Configuration,
		task_manager: &'a mut TaskManager,
		client: Arc<C>,
	) -> Result<(), FrostWorkerError>
	where
		P: Pair + Send + Sync,
		P::Public: AppPublic + Hash + Member + Encode + Decode,
		P::Signature: TryFrom<Vec<u8>> + Hash + Member + Encode + Decode,
		B: BlockT,
		C: ProvideRuntimeApi<B> + BlockOf + AuxStore + HeaderBackend<B> + 'static,
		C::Api: AuraApi<B, AuthorityId<P>>,
	{
		log::warn!("================================================");
		log::warn!("warn log start_frost_worker");

		let keystore_path = config.keystore.path();
		let chain_type = config.chain_spec.chain_type();

		log::warn!("keystore path {:#?}", keystore_path);
		log::warn!("chain type {:#?}", chain_type);

		task_manager
			.spawn_handle()
			.spawn_blocking("FrostWorkerEvents", None, async move {
				log::warn!("Inside first task");
				// We listen to the event that is emitted by the call that is called below
				// The emitted data if successfull will contain the cluster ids for which there
				// is no group id and the length is 11 (i.e cluster is complete)
				let res = listen_to_event().await;

				match res {
					Ok(v) => v,
					Err(e) => {
						log::warn!("FAILED IN LISTEN TO EVENT WITH ERROR {:#?}", e)
					},
				};
			});

		log::warn!("between tasks");

		task_manager
			.spawn_handle()
			.spawn_blocking("FrostWorkerEmitter", None, async move {
				let runtime_api = client.runtime_api();
				log::warn!("Inside second task");

				loop {
					thread::sleep(Duration::from_secs(30));

					let best_hash = client.info().best_hash;
					let res = runtime_api.emit_ids_of_clusters_without_group_keys(best_hash);

					match res {
						Ok(v) => {
							log::warn!("emit_ids_of_clusters_without_group_keys VALUE {:#?}", &v);
							v
						},
						Err(e) => {
							log::warn!("emit_ids_of_clusters_without_group_keys ERROR {:#?}", e);
							continue;
						},
					};
				}
			});

		log::warn!("================================================");
		Ok(())
	}

	async fn create_client() -> Result<OnlineClient<PolkadotConfig>, subxt::Error> {
		log::info!("create_client");

		thread::sleep(Duration::from_secs(20));

		let api = OnlineClient::<PolkadotConfig>::new().await;
		let api = match api {
			Ok(v) => v,
			Err(e) => {
				log::warn!("FAILED TO CREATE CLIENT WITH ERROR {:#?}", e);
				return Err(e);
			},
		};
		Ok(api)
	}

	async fn listen_to_event() -> Result<(), FrostWorkerError> {
		log::info!("listen_to_event");

		let api = create_client().await.or(Err(FrostWorkerError::FailedToCreateSubstrateClient))?;

		let mut blocks = api
			.blocks()
			.subscribe_finalized()
			.await
			.or(Err(FrostWorkerError::FailedToSubscribeToBlocks))?;

		// Ignore errors and move to the next iteration
		// We are ignoring errors because we will retry after 30 from_secs and
		// the events will be emitted again
		while let Some(block) = blocks.next().await {
			let block = match block {
				Ok(v) => v,
				Err(_) => continue,
			};

			let events = block.events().await;

			let events = match events {
				Ok(v) => v,
				Err(_) => continue,
			};

			let decoded_event =
				events.find_first::<bridge::clusters::events::GetIdsOfClustersWithoutGroupKey>();

			let decoded_event = match decoded_event {
				Ok(v) => v,
				Err(_) => continue,
			};

			// Start the key generation process
			println!("Decoded Event {:#?}", decoded_event);
		}

		log::warn!("======--===========--========--=========--");
		log::warn!("SHOULD ONLY RUN ON ERROR");

		Ok(())
	}
}
