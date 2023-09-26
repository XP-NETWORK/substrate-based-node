/// Module contains functions related to frost
pub mod frost {
	use std::{
		fs::{self, File},
		time::Duration,
	};

	use futures::StreamExt;
	use sc_network::config::FullNetworkConfiguration;
	use sc_service::Configuration;
	use sp_core::sr25519::Pair;
	use subxt::{
		client::OnlineClientT, ext::scale_encode::EncodeAsFields, tx::Payload, OnlineClient,
		PolkadotConfig,
	};
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

	#[subxt::subxt(runtime_metadata_path = "./src/metadata.scale")]
	pub mod bridge {}

	/// Start a seperate worker for continously check for any cluster without group key
	/// If theres a compelete cluster i.e a cluster has 11 validators and group key is not generated for i
	/// then start the group key generation.
	pub fn start_frost_worker(config: Configuration) -> Result<(), FrostWorkerError> {
		let _ = tokio::task::spawn_blocking(|| async move {
			// We listen to the event that is emitted by the call that is called below
			// The emitted data if successfull will contain the cluster ids for which there
			// is no group id and the length is 11 (i.e cluster is complete)
			let _ = listen_to_event().await;
		});

		let _ = tokio::task::spawn_blocking(move || async move {
			loop {
				let call = bridge::tx().clusters().emit_ids_of_clusters_without_group_keys();

				// ignore err if there is any and try again after sometime
				let _ = sign_and_send_substrate_tx(call, &config).await;
				let _ = tokio::time::sleep(Duration::from_secs(30));
			}
		});

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
		let api = OnlineClient::<PolkadotConfig>::new().await?;
		Ok(api)
	}

	async fn listen_to_event() -> Result<(), FrostWorkerError> {
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
		Ok(())
	}

	async fn sign_and_send_substrate_tx<T: EncodeAsFields>(
		call: Payload<T>,
		config: &Configuration,
	) -> Result<(), FrostWorkerError> {
		let this_validator_keypair = create_substrate_signer(config)?;

		let api = create_client().await.or(Err(FrostWorkerError::FailedToCreateSubstrateClient))?;

		let _ = api
			.tx()
			.sign_and_submit_then_watch_default(&call, &this_validator_keypair)
			.await
			.or(Err(FrostWorkerError::FailedToSubmitTransaction))?
			.wait_for_finalized_success()
			.await
			.or(Err(FrostWorkerError::SubstrateTransactionFailed))?;

		Ok(())
	}
}
