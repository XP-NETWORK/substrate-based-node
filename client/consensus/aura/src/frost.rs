/// Module contains functions related to frost
pub mod frost {
	use std::{
		fs::{self},
		thread::{self, sleep},
		time::Duration,
	};

	use futures::StreamExt;
	use sc_service::{Configuration, TaskManager};
	use subxt::{ext::scale_encode::EncodeAsFields, tx::Payload, OnlineClient, PolkadotConfig};
	use subxt_signer::{bip39::Mnemonic, sr25519::Keypair};

	/// The error enum for Frost Worker
	pub enum FrostWorkerError {
		/// Failure in submitting tx to the substrate runtime
		FailedToSubmitTransaction,
		/// Submitted tx to substrate runtime failed
		SubstrateTransactionFailed,
		/// FailedToSubscribeToBlocks
		FailedToSubscribeToBlocks,
		///
		FailedToCreateSubstrateClient,
		///
		FailedToReadMnemonic,
	}

	// task_manager.spawn_handle().spawn_blocking("test", None, async move {
	#[subxt::subxt(runtime_metadata_path = "./src/metadata.scale")]
	pub mod bridge {}

	/// Start a seperate worker for continously check for any cluster without group key
	/// If theres a compelete cluster i.e a cluster has 11 validators and group key is not generated for i
	/// then start the group key generation.
	pub fn start_frost_worker<'a>(
		config: &'a Configuration,
		task_manager: &'a mut TaskManager,
	) -> Result<(), FrostWorkerError> {
		log::info!("info start_frost_worker");
		// log::log!("log start_frost_worker");
		log::warn!("warn log start_frost_worker");
		log::error!("error log start_frost_worker");
		log::trace!("trace log start_frost_worker");

		let this_validator_keypair = create_substrate_signer(config)?;

		// let t = task_manager.spawn_handle().spawn_blocking("test", None, async move {
		// 	log::info!("Inside first task");
		// 	log::warn!("Inside first task");
		// 	log::error!("Inside first task");
		// 	log::trace!("Inside first task");
		// 	// We listen to the event that is emitted by the call that is called below
		// 	// The emitted data if successfull will contain the cluster ids for which there
		// 	// is no group id and the length is 11 (i.e cluster is complete)
		// 	let _ = listen_to_event().await;
		// 	// sleep(Duration::from_secs(60));
		// 	loop {}
		// 	// return b"return";
		// });

		// let handle = thread::spawn(|| {
		// 	log::info!("Inside first task");
		// 	log::warn!("Inside first task");
		// 	log::error!("Inside first task");
		// 	log::trace!("Inside first task");
		// We listen to the event that is emitted by the call that is called below
		// The emitted data if successfull will contain the cluster ids for which there
		// is no group id and the length is 11 (i.e cluster is complete)
		let _ = listen_to_event();
		// });

		// let _ = handle.join().unwrap();

		log::info!("after joining the handle");

		let _ = tokio::task::spawn_blocking(move || async move {
			loop {
				let call = bridge::tx().clusters().emit_ids_of_clusters_without_group_keys();

				// ignore err if there is any and try again after sometime
				let _ = sign_and_send_substrate_tx(call, &this_validator_keypair).await;
				let _ = tokio::time::sleep(Duration::from_secs(30));
			}
		});

		log::info!("BEFORE LOOP");

		loop {}
		Ok(())
	}

	fn get_path(config: &Configuration) -> std::path::PathBuf {
		let base_path = &config.base_path;
		let config_dir =
			base_path.config_dir(config.chain_spec.id()).join("keystore").join("passphrase");
		config_dir
	}

	fn read_phrase(path: &str) -> Result<String, FrostWorkerError> {
		let contents = fs::read_to_string(path).or(Err(FrostWorkerError::FailedToReadMnemonic))?;
		Ok(contents)
	}

	fn get_passphrase(config: &Configuration) -> Result<String, FrostWorkerError> {
		let path = get_path(config);
		let read_phrase = read_phrase(&path.to_str().unwrap())?;
		Ok(read_phrase)
	}

	fn create_substrate_signer(config: &Configuration) -> Result<Keypair, FrostWorkerError> {
		let passphrase = get_passphrase(config)?;
		let mnemonic = Mnemonic::parse(passphrase).unwrap();
		let keypair = Keypair::from_phrase(&mnemonic, None).unwrap();
		Ok(keypair)
	}

	async fn create_client() -> Result<OnlineClient<PolkadotConfig>, subxt::Error> {
		log::info!("create_client");
		let api = OnlineClient::<PolkadotConfig>::new().await?;
		Ok(api)
	}

	async fn listen_to_event() -> Result<(), FrostWorkerError> {
		log::info!("listen_to_event");
		let api = create_client().await.or(Err(FrostWorkerError::FailedToCreateSubstrateClient))?;

		log::info!("listen_to_event api {:#?}", &api);
		let mut blocks = api
			.blocks()
			.subscribe_finalized()
			.await
			.or(Err(FrostWorkerError::FailedToSubscribeToBlocks))?;

		log::info!("after listen_to_event");
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
		Ok(())
	}

	async fn sign_and_send_substrate_tx<T: EncodeAsFields>(
		call: Payload<T>,
		this_validator_keypair: &Keypair,
	) -> Result<(), FrostWorkerError> {
		log::info!("sign_and_send_substrate_tx");
		let api = create_client().await.or(Err(FrostWorkerError::FailedToCreateSubstrateClient))?;

		log::info!("sign_and_send_substrate_tx api {:#?}", &api);
		let _ = api
			.tx()
			.sign_and_submit_then_watch_default(&call, this_validator_keypair)
			.await
			.or(Err(FrostWorkerError::FailedToSubmitTransaction))?
			.wait_for_finalized_success()
			.await
			.or(Err(FrostWorkerError::SubstrateTransactionFailed))?;

		Ok(())
	}
}
