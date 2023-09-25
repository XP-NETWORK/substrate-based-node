/// Module contains functions related to frost
pub mod frost {
	use std::thread;
	/// Start a seperate worker for continously check for any cluster without group key
	/// If theres a compelete cluster i.e a cluster has 11 validators and group key is not generated for i
	/// then start the group key generation.
	pub fn start_frost_worker<AN>(network: AN) {
		let _ = thread::Builder
			::new()
			.name("Frost Worker".to_string())
			.spawn(move || {
				loop {
					// check for clusters without group keys
					todo!();
				}
			});
	}
}
