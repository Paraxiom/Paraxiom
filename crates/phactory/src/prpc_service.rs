use std::convert::TryFrom;
use std::future::Future;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::benchmark::Flags;
use crate::hex;
use crate::system::{System, MAX_SUPPORTED_CONSENSUS_VERSION};

use super::*;
use crate::contracts::ContractClusterId;
use ::pink::runtime::ExecSideEffects;
use parity_scale_codec::Encode;
use pb::{
    phactory_api_server::{PhactoryApi, PhactoryApiServer},
    server::Error as RpcError,
};
use phactory_api::blocks::StorageState;
use phactory_api::{blocks, crypto, endpoints::EndpointType, prpc as pb};
use phala_crypto::{
    key_share,
    sr25519::{Persistence, KDF},
};
use phala_pallets::utils::attestation::{validate as validate_attestation_report, IasFields};
use phala_types::contract::contract_id_preimage;
use phala_types::{
    contract, messaging::EncryptedKey, wrap_content_to_sign, AttestationReport,
    ChallengeHandlerInfo, EncryptedWorkerKey, SignedContentType, VersionedWorkerEndpoints,
    WorkerEndpointPayload, WorkerPublicKey, WorkerRegistrationInfoV2,
};
use tokio::sync::oneshot::{channel, Sender};

type RpcResult<T> = Result<T, RpcError>;

fn from_display(e: impl core::fmt::Display) -> RpcError {
    RpcError::AppError(e.to_string())
}

fn from_debug(e: impl core::fmt::Debug) -> RpcError {
    RpcError::AppError(format!("{e:?}"))
}

fn now() -> u64 {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    now.as_secs()
}

impl<Platform: pal::Platform + Serialize + DeserializeOwned> Phactory<Platform> {
    fn runtime_state(&mut self) -> RpcResult<&mut RuntimeState> {
        self.runtime_state
            .as_mut()
            .ok_or_else(|| from_display("Runtime not initialized"))
    }

    fn system(&mut self) -> RpcResult<&mut System<Platform>> {
        self.system
            .as_mut()
            .ok_or_else(|| from_display("Runtime not initialized"))
    }

    pub fn get_info(&self) -> pb::PhactoryInfo {
        let initialized = self.system.is_some();
        let state = self.runtime_state.as_ref();
        let genesis_block_hash = state.map(|state| hex::encode(state.genesis_block_hash));
        let dev_mode = self.dev_mode;

        let (state_root, pending_messages, counters) = match state.as_ref() {
            Some(state) => {
                let state_root = hex::encode(state.chain_storage.root());
                let pending_messages = state.send_mq.count_messages();
                let counters = state.storage_synchronizer.counters();
                (state_root, pending_messages, counters)
            }
            None => Default::default(),
        };

        let system_info = self.system.as_ref().map(|s| s.get_info());
        let score = benchmark::score();
        let m_usage = self.platform.memory_usage();

        // Deprecated fields
        let registered;
        let public_key;
        let ecdh_public_key;
        let gatekeeper;
        match &system_info {
            None => {
                registered = false;
                public_key = None;
                ecdh_public_key = None;
                gatekeeper = Some(pb::GatekeeperStatus {
                    role: pb::GatekeeperRole::None.into(),
                    master_public_key: Default::default(),
                });
            }
            Some(sys) => {
                registered = sys.registered;
                public_key = Some(sys.public_key.clone());
                ecdh_public_key = Some(sys.ecdh_public_key.clone());
                gatekeeper = sys.gatekeeper.clone();
            }
        };

        pb::PhactoryInfo {
            initialized,
            registered,
            public_key,
            ecdh_public_key,
            gatekeeper,
            genesis_block_hash,
            headernum: counters.next_header_number,
            para_headernum: counters.next_para_header_number,
            blocknum: counters.next_block_number,
            waiting_for_paraheaders: counters.waiting_for_paraheaders,
            state_root,
            dev_mode,
            pending_messages: pending_messages as _,
            score,
            version: self.args.version.clone(),
            git_revision: self.args.git_revision.clone(),
            memory_usage: Some(pb::MemoryUsage {
                rust_used: m_usage.rust_used as _,
                rust_peak_used: m_usage.rust_peak_used as _,
                total_peak_used: m_usage.total_peak_used as _,
            }),
            system: system_info,
            can_load_chain_state: self.can_load_chain_state,
        }
    }

    pub(crate) fn sync_header(
        &mut self,
        headers: Vec<blocks::HeaderToSync>,
        authority_set_change: Option<blocks::AuthoritySetChange>,
    ) -> RpcResult<pb::SyncedTo> {
        info!(
            "sync_header from={:?} to={:?}",
            headers.first().map(|h| h.header.number),
            headers.last().map(|h| h.header.number)
        );
        self.can_load_chain_state = false;
        let last_header = self
            .runtime_state()?
            .storage_synchronizer
            .sync_header(headers, authority_set_change)
            .map_err(from_display)?;

        Ok(pb::SyncedTo {
            synced_to: last_header,
        })
    }

    pub(crate) fn sync_para_header(
        &mut self,
        headers: blocks::Headers,
        proof: blocks::StorageProof,
    ) -> RpcResult<pb::SyncedTo> {
        info!(
            "sync_para_header from={:?} to={:?}",
            headers.first().map(|h| h.number),
            headers.last().map(|h| h.number)
        );

        let state = self.runtime_state()?;

        let storage_key = light_validation::utils::storage_map_prefix_twox_64_concat(
            b"Paras",
            b"Heads",
            &state.para_id,
        );

        let last_header = state
            .storage_synchronizer
            .sync_parachain_header(headers, proof, &storage_key)
            .map_err(from_display)?;

        Ok(pb::SyncedTo {
            synced_to: last_header,
        })
    }

    /// Sync a combined batch of relaychain & parachain headers
    /// NOTE:
    ///   - The two latest headers MUST be aligned with each other by the `Para.Heads` read from the relaychain storage.
    ///   - The operation is not guarenteed to be atomical. If the parachain header is rejected, the already synced relaychain
    ///     headers will keep it's progress.
    pub(crate) fn sync_combined_headers(
        &mut self,
        relaychain_headers: Vec<blocks::HeaderToSync>,
        authority_set_change: Option<blocks::AuthoritySetChange>,
        parachain_headers: blocks::Headers,
        proof: blocks::StorageProof,
    ) -> RpcResult<pb::HeadersSyncedTo> {
        let relaychain_synced_to = self
            .sync_header(relaychain_headers, authority_set_change)?
            .synced_to;
        let parachain_synced_to = if parachain_headers.is_empty() {
            self.runtime_state()?
                .storage_synchronizer
                .counters()
                .next_para_header_number
                - 1
        } else {
            self.sync_para_header(parachain_headers, proof)?.synced_to
        };
        Ok(pb::HeadersSyncedTo {
            relaychain_synced_to,
            parachain_synced_to,
        })
    }

    pub(crate) fn dispatch_block(
        &mut self,
        mut blocks: Vec<blocks::BlockHeaderWithChanges>,
    ) -> RpcResult<pb::SyncedTo> {
        info!(
            "dispatch_block from={:?} to={:?}",
            blocks.first().map(|h| h.block_header.number),
            blocks.last().map(|h| h.block_header.number)
        );
        let counters = self.runtime_state()?.storage_synchronizer.counters();
        blocks.retain(|b| b.block_header.number >= counters.next_block_number);

        let mut last_block = counters.next_block_number - 1;
        for block in blocks.into_iter() {
            info!("Dispatching block: {}", block.block_header.number);
            let state = self.runtime_state()?;
            state
                .storage_synchronizer
                .feed_block(&block, state.chain_storage.inner_mut())
                .map_err(from_display)?;
            info!("State synced");
            state.purge_mq();
            self.check_requirements();
            self.handle_inbound_messages(block.block_header.number)?;
            last_block = block.block_header.number;

            if let Err(e) = self.maybe_take_checkpoint(last_block) {
                error!("Failed to take checkpoint: {:?}", e);
            }
        }

        Ok(pb::SyncedTo {
            synced_to: last_block,
        })
    }

    fn maybe_take_checkpoint(&mut self, current_block: chain::BlockNumber) -> anyhow::Result<()> {
        if !self.args.enable_checkpoint {
            return Ok(());
        }
        if self.last_checkpoint.elapsed().as_secs() < self.args.checkpoint_interval {
            return Ok(());
        }
        self.take_checkpoint(current_block)
    }

    #[allow(clippy::too_many_arguments)]
    fn init_runtime(
        &mut self,
        is_parachain: bool,
        genesis: blocks::GenesisBlockInfo,
        genesis_state: blocks::StorageState,
        operator: Option<chain::AccountId>,
        debug_set_key: ::core::option::Option<Vec<u8>>,
        attestation_provider: ::core::option::Option<AttestationProvider>,
    ) -> RpcResult<pb::InitRuntimeResponse> {
        if self.system.is_some() {
            return Err(from_display("Runtime already initialized"));
        }

        info!("Initializing runtime");
        info!("is_parachain  : {is_parachain}");
        info!("operator      : {operator:?}");
        info!("ra_provider   : {attestation_provider:?}");
        info!("debug_set_key : {debug_set_key:?}");

        // load chain genesis
        let genesis_block_hash = genesis.block_header.hash();
        let chain_storage = ChainStorage::from_pairs(genesis_state.into_iter());
        let para_id = chain_storage.para_id();
        info!(
            "Genesis state loaded: root={:?}, para_id={para_id}",
            chain_storage.root()
        );

        // load identity
        let rt_data = if let Some(raw_key) = debug_set_key {
            let priv_key = sr25519::Pair::from_seed_slice(&raw_key).map_err(from_debug)?;
            self.init_runtime_data(genesis_block_hash, para_id, Some(priv_key))
                .map_err(from_debug)?
        } else {
            self.init_runtime_data(genesis_block_hash, para_id, None)
                .map_err(from_debug)?
        };
        self.dev_mode = rt_data.dev_mode;

        self.attestation_provider = attestation_provider;
        info!("attestation_provider: {:?}", self.attestation_provider);

        if self.dev_mode && self.attestation_provider.is_some() {
            return Err(from_display(
                "RA is disallowed when debug_set_key is enabled",
            ));
        }

        self.platform
            .quote_test(self.attestation_provider)
            .map_err(from_debug)?;

        let (identity_key, ecdh_key) = rt_data.decode_keys();

        let ecdsa_pk = identity_key.public();
        let ecdsa_hex_pk = hex::encode(ecdsa_pk);
        info!("Identity pubkey: {:?}", ecdsa_hex_pk);

        // derive ecdh key
        let ecdh_pubkey = phala_types::EcdhPublicKey(ecdh_key.public());
        let ecdh_hex_pk = hex::encode(ecdh_pubkey.0.as_ref());
        info!("ECDH pubkey: {:?}", ecdh_hex_pk);

        ::pink::runtime::set_worker_pubkey(ecdh_pubkey.0);

        // Measure machine score
        let cpu_core_num: u32 = self.platform.cpu_core_num();
        info!("CPU cores: {}", cpu_core_num);

        let cpu_feature_level: u32 = self.platform.cpu_feature_level();
        info!("CPU feature level: {}", cpu_feature_level);

        // Initialize bridge
        let next_headernum = genesis.block_header.number + 1;
        let mut light_client = LightValidation::new();
        let main_bridge = light_client
            .initialize_bridge(
                genesis.block_header.clone(),
                genesis.authority_set,
                genesis.proof,
            )
            .expect("Bridge initialize failed");

        let storage_synchronizer = if is_parachain {
            Synchronizer::new_parachain(light_client, main_bridge, next_headernum)
        } else {
            Synchronizer::new_solochain(light_client, main_bridge)
        };

        let send_mq = MessageSendQueue::default();
        let recv_mq = MessageDispatcher::default();

        let contracts = contracts::ContractsKeeper::default();

        let mut runtime_state = RuntimeState {
            send_mq,
            recv_mq,
            storage_synchronizer,
            chain_storage,
            genesis_block_hash,
            para_id,
        };

        // In parachain mode the state root is stored in parachain header which isn't passed in here.
        // The storage root would be checked at the time each block being synced in(where the storage
        // being patched) and pRuntime doesn't read any data from the chain storage until the first
        // block being synced in. So it's OK to skip the check here.
        if !is_parachain && *runtime_state.chain_storage.root() != genesis.block_header.state_root {
            error!(
                "Genesis state root mismatch, required in header: {:?}, actual: {:?}",
                genesis.block_header.state_root,
                runtime_state.chain_storage.root(),
            );
            return Err(from_display("state root mismatch"));
        }

        let system = system::System::new(
            self.platform.clone(),
            self.dev_mode,
            self.args.sealing_path.clone(),
            self.args.storage_path.clone(),
            identity_key,
            ecdh_key,
            rt_data.trusted_sk,
            &runtime_state.send_mq,
            &mut runtime_state.recv_mq,
            contracts,
            self.args.cores as _,
        );

        // Build WorkerRegistrationInfoV2
        let runtime_info = WorkerRegistrationInfoV2::<chain::AccountId> {
            version: Self::compat_app_version(),
            machine_id: self.machine_id.clone(),
            pubkey: ecdsa_pk,
            ecdh_pubkey,
            genesis_block_hash,
            features: vec![cpu_core_num, cpu_feature_level],
            operator,
            para_id,
            max_consensus_version: MAX_SUPPORTED_CONSENSUS_VERSION,
        };

        let resp = pb::InitRuntimeResponse::new(
            runtime_info,
            genesis_block_hash,
            ecdsa_pk,
            ecdh_pubkey,
            None,
        );
        self.runtime_info = Some(resp.clone());
        self.runtime_state = Some(runtime_state);
        self.system = Some(system);
        Ok(resp)
    }

    fn get_runtime_info(
        &mut self,
        refresh_ra: bool,
        operator: Option<chain::AccountId>,
    ) -> RpcResult<pb::InitRuntimeResponse> {
        let validated_identity_key = self.system()?.validated_identity_key();
        let validated_state = self.runtime_state()?.storage_synchronizer.state_validated();

        let reset_operator = operator.is_some();
        if reset_operator {
            self.update_runtime_info(move |info| {
                info.operator = operator;
            });
        }

        let mut cached_resp = self
            .runtime_info
            .as_mut()
            .ok_or_else(|| from_display("Uninitiated runtime info"))?;

        if let Some(cached_attestation) = &cached_resp.attestation {
            const MAX_ATTESTATION_AGE: u64 = 60 * 60;
            if refresh_ra
                || reset_operator
                || now() > cached_attestation.timestamp + MAX_ATTESTATION_AGE
            {
                cached_resp.attestation = None;
            }
        }

        let allow_attestation =
            validated_state && (validated_identity_key || self.attestation_provider.is_none());
        info!("validated_identity_key :{validated_identity_key}");
        info!("validated_state        :{validated_state}");
        info!("refresh_ra             :{refresh_ra}");
        info!("reset_operator         :{reset_operator}");
        info!("attestation_provider   :{:?}", self.attestation_provider);
        info!("allow_attestation      :{allow_attestation}");
        // Never generate RA report for a potentially injected identity key
        // else he is able to fake a Secure Worker
        if allow_attestation && cached_resp.attestation.is_none() {
            // We hash the encoded bytes directly
            let runtime_info_hash = sp_core::hashing::blake2_256(&cached_resp.encoded_runtime_info);
            info!("Encoded runtime info");
            info!("{:?}", hex::encode(&cached_resp.encoded_runtime_info));

            let encoded_report = match self
                .platform
                .create_attestation_report(self.attestation_provider, &runtime_info_hash)
            {
                Ok(r) => r,
                Err(e) => {
                    let message = format!("Failed to create attestation report: {e:?}");
                    error!("{}", message);
                    return Err(from_display(message));
                }
            };

            cached_resp.attestation = Some(pb::Attestation {
                version: 1,
                provider: serde_json::to_string(&self.attestation_provider).unwrap(),
                payload: None,
                encoded_report,
                timestamp: now(),
            });
        }

        Ok(cached_resp.clone())
    }

    fn get_egress_messages(&mut self) -> RpcResult<pb::EgressMessages> {
        let messages: Vec<_> = self
            .runtime_state
            .as_ref()
            .map(|state| state.send_mq.all_messages_grouped().into_iter().collect())
            .unwrap_or_default();
        Ok(messages)
    }

    fn apply_side_effects(&mut self, cluster_id: ContractClusterId, effects: ExecSideEffects) {
        let Some(state) = self.runtime_state.as_ref() else {
            error!("Failed to apply side effects: chain storage missing");
            return;
        };
        let Some(system) = self.system.as_mut() else {
            error!("Failed to apply side effects: system missing");
            return;
        };
        system.apply_side_effects(cluster_id, effects, &state.chain_storage);
    }

    fn contract_query(
        &mut self,
        request: pb::ContractQueryRequest,
        effects_queue: Sender<(ContractClusterId, ExecSideEffects)>,
    ) -> RpcResult<impl Future<Output = RpcResult<pb::ContractQueryResponse>>> {
        // Validate signature
        let origin = if let Some(sig) = &request.signature {
            let current_block = self.get_info().blocknum - 1;
            // At most two level cert chain supported
            match sig.verify(&request.encoded_encrypted_data, current_block, 2) {
                Ok(key_chain) => match &key_chain[..] {
                    [root_pubkey, ..] => Some(root_pubkey.clone()),
                    _ => {
                        return Err(from_display("BUG: verify ok but no key?"));
                    }
                },
                Err(err) => {
                    return Err(from_display(format!(
                        "Verifying signature failed: {:?}",
                        err
                    )));
                }
            }
        } else {
            info!("No query signature");
            None
        };

        debug!("Verifying signature passed! origin={:?}", origin);

        let ecdh_key = self.system()?.ecdh_key.clone();

        // Decrypt data
        let encrypted_req = request.decode_encrypted_data()?;
        let data = encrypted_req.decrypt(&ecdh_key).map_err(from_debug)?;

        // Decode head
        let mut data_cursor = &data[..];
        let head = contract::ContractQueryHead::decode(&mut data_cursor)?;
        let rest = data_cursor.len();

        // Origin
        let accid_origin = match origin {
            Some(origin) => {
                let accid = chain::AccountId::try_from(origin.as_slice())
                    .map_err(|_| from_display("Bad account id"))?;
                Some(accid)
            }
            None => None,
        };

        let query_scheduler = self.query_scheduler.clone();
        // Dispatch
        let query_future = self.system()?.make_query(
            &head.id,
            accid_origin.as_ref(),
            data[data.len() - rest..].to_vec(),
            query_scheduler,
        )?;

        Ok(async move {
            let (response, cluster_id, effects) = query_future.await?;

            effects_queue
                .send((cluster_id, effects))
                .map_err(|_| from_display("Failed to apply side effects"))?;

            let response = contract::ContractQueryResponse {
                nonce: head.nonce,
                result: contract::Data(response),
            };
            let response_data = response.encode();

            // Encrypt
            let encrypted_resp = crypto::EncryptedData::encrypt(
                &ecdh_key,
                &encrypted_req.pubkey,
                crate::generate_random_iv(),
                &response_data,
            )
            .map_err(from_debug)?;

            Ok(pb::ContractQueryResponse::new(encrypted_resp))
        })
    }

    fn handle_inbound_messages(&mut self, block_number: chain::BlockNumber) -> RpcResult<()> {
        let state = self
            .runtime_state
            .as_mut()
            .ok_or_else(|| from_display("Runtime not initialized"))?;
        let system = self
            .system
            .as_mut()
            .ok_or_else(|| from_display("Runtime not initialized"))?;

        // Dispatch events
        let messages = state
            .chain_storage
            .mq_messages()
            .map_err(|_| from_display("Can not get mq messages from storage"))?;

        state.recv_mq.reset_local_index();

        let now_ms = state.chain_storage.timestamp_now();

        let mut block = BlockInfo {
            block_number,
            now_ms,
            storage: &state.chain_storage,
            send_mq: &state.send_mq,
            recv_mq: &mut state.recv_mq,
        };

        system.will_process_block(&mut block);

        for message in messages {
            use phala_types::messaging::SystemEvent;
            macro_rules! log_message {
                ($msg: expr, $t: ident) => {{
                    let event: Result<$t, _> =
                        parity_scale_codec::Decode::decode(&mut &$msg.payload[..]);
                    match event {
                        Ok(event) => {
                            debug!(target: "phala_mq",
                                "mq dispatching message: sender={} dest={:?} payload={:?}",
                                $msg.sender, $msg.destination, event
                            );
                        }
                        Err(_) => {
                            debug!(target: "phala_mq", "mq dispatching message (decode failed): {:?}", $msg);
                        }
                    }
                }};
            }
            // TODO.kevin: reuse codes in debug-cli
            if message.destination.path() == &SystemEvent::topic() {
                log_message!(message, SystemEvent);
            } else {
                debug!(target: "phala_mq",
                    "mq dispatching message: sender={}, dest={:?}",
                    message.sender, message.destination
                );
            }
            block.recv_mq.dispatch(message);

            system.process_messages(&mut block);
        }
        system.did_process_block(&mut block);

        let n_unhandled = block.recv_mq.clear();
        if n_unhandled > 0 {
            warn!("There are {} unhandled messages dropped", n_unhandled);
        }

        let block_time = now_ms / 1000;
        let sys_time = now();

        // When delta time reaches 3600s, there are about 3600 / 12 = 300 blocks rest.
        // It need about 30 more seconds to sync up to date.
        let syncing = block_time + 3600 <= sys_time;
        debug!(
            "block_time={}, sys_time={}, syncing={}",
            block_time, sys_time, syncing
        );
        benchmark::set_flag(Flags::SYNCING, syncing);

        Ok(())
    }

    fn add_endpoint(
        &mut self,
        endpoint_type: EndpointType,
        endpoint: String,
    ) -> RpcResult<pb::GetEndpointResponse> {
        self.endpoints.insert(endpoint_type, endpoint);
        self.sign_endpoints()
    }

    fn sign_endpoints(&mut self) -> RpcResult<pb::GetEndpointResponse> {
        let system = self.system()?;
        let block_time: u64 = system.now_ms;
        let public_key = system.identity_key.public();
        let versioned_endpoints =
            VersionedWorkerEndpoints::V1(self.endpoints.values().cloned().collect());
        let endpoint_payload = WorkerEndpointPayload {
            pubkey: public_key,
            versioned_endpoints,
            signing_time: block_time,
        };
        let signature = self.sign_endpoint_payload(&endpoint_payload)?;
        let resp = pb::GetEndpointResponse::new(Some(endpoint_payload.clone()), Some(signature));
        self.signed_endpoints = Some(resp.clone());
        Ok(resp)
    }

    fn get_endpoint_info(&mut self) -> RpcResult<pb::GetEndpointResponse> {
        if self.endpoints.is_empty() {
            info!("Endpoint not found");
            return Ok(pb::GetEndpointResponse::new(None, None));
        }
        match &self.signed_endpoints {
            Some(response) => Ok(response.clone()),
            None => self.sign_endpoints(),
        }
    }

    fn sign_endpoint_info(
        &mut self,
        versioned_endpoints: VersionedWorkerEndpoints,
    ) -> RpcResult<pb::GetEndpointResponse> {
        let system = self.system()?;
        let pubkey = system.identity_key.public();
        let signing_time = system.now_ms;
        let payload = WorkerEndpointPayload {
            pubkey,
            versioned_endpoints,
            signing_time,
        };
        let signature = self.sign_endpoint_payload(&payload)?;

        Ok(pb::GetEndpointResponse::new(Some(payload), Some(signature)))
    }

    fn sign_endpoint_payload(&mut self, payload: &WorkerEndpointPayload) -> RpcResult<Vec<u8>> {
        const MAX_PAYLOAD_SIZE: usize = 2048;
        let data_to_sign = payload.encode();
        if data_to_sign.len() > MAX_PAYLOAD_SIZE {
            return Err(from_display("Endpoints too large"));
        }
        let wrapped_data = wrap_content_to_sign(&data_to_sign, SignedContentType::EndpointInfo);
        let signature = self
            .system()?
            .identity_key
            .clone()
            .sign(&wrapped_data)
            .encode();
        Ok(signature)
    }

    pub fn get_contract_info(
        &mut self,
        contract_ids: &[String],
    ) -> RpcResult<pb::GetContractInfoResponse> {
        // TODO: use `let else`
        let system = match &self.system {
            None => return Ok(Default::default()),
            Some(system) => system,
        };
        let contracts = if contract_ids.is_empty() {
            system
                .contracts
                .iter()
                .map(|(_, contract)| contract.info())
                .collect()
        } else {
            let mut contracts = vec![];
            for id in contract_ids.iter() {
                let raw: [u8; 32] = try_decode_hex(id)
                    .map_err(|_| from_display("Invalid contract id"))?
                    .try_into()
                    .map_err(|_| from_display("Invalid contract id"))?;
                let contract = system.contracts.get(&raw.into());
                // TODO: use `let else`.
                let contract = match contract {
                    None => continue,
                    Some(contract) => contract,
                };
                contracts.push(contract.info());
            }
            contracts
        };
        Ok(pb::GetContractInfoResponse { contracts })
    }

    pub fn get_cluster_info(&self) -> RpcResult<pb::GetClusterInfoResponse> {
        // TODO: use `let else`.
        let system = match &self.system {
            None => return Ok(Default::default()),
            Some(system) => system,
        };
        let clusters = system
            .contract_clusters
            .iter()
            .map(|(id, cluster)| {
                let contracts = cluster.iter_contracts().map(hex).collect();
                let ver = cluster.config.version;
                let version = format!("{}.{}", ver.0, ver.1);
                pb::ClusterInfo {
                    id: hex(id),
                    state_root: hex(cluster.storage.root()),
                    contracts,
                    version,
                }
            })
            .collect();
        Ok(pb::GetClusterInfoResponse { clusters })
    }

    pub fn upload_sidevm_code(&mut self, contract_id: ContractId, code: Vec<u8>) -> RpcResult<()> {
        self.system()?
            .upload_sidevm_code(contract_id, code)
            .map_err(from_display)
    }

    pub fn load_chain_state(
        &mut self,
        block: chain::BlockNumber,
        storage: StorageState,
    ) -> anyhow::Result<()> {
        if !self.can_load_chain_state {
            anyhow::bail!("Can not load chain state");
        }
        if block == 0 {
            anyhow::bail!("Can not load chain state from block 0");
        }
        let Some(system) = &mut self.system else {
            anyhow::bail!("System is uninitialized");
        };
        let Some(state) = &mut self.runtime_state else {
            anyhow::bail!("Runtime is uninitialized");
        };
        let chain_storage = ChainStorage::from_pairs(storage.into_iter());
        let para_id = chain_storage.para_id();
        if para_id != state.para_id {
            anyhow::bail!(
                "Loading different parachain state, loading={para_id}, init={}",
                state.para_id
            );
        }
        if chain_storage.is_worker_registered(&system.identity_key.public()) {
            anyhow::bail!(
                "Failed to load state: the worker is already registered at block {block}",
            );
        }
        state
            .storage_synchronizer
            .assume_at_block(block)
            .context("Failed to set synchronizer state")?;
        state.chain_storage = chain_storage;
        system.genesis_block = block;
        self.can_load_chain_state = false;
        Ok(())
    }

    pub fn stop(&self, remove_checkpoints: bool) -> RpcResult<()> {
        info!("Requested to stop remove_checkpoints={remove_checkpoints}");
        if remove_checkpoints {
            crate::maybe_remove_checkpoints(&self.args.sealing_path);
        }
        std::process::abort()
    }
}

#[derive(Clone)]
pub struct RpcService<Platform> {
    pub(crate) phactory: Arc<Mutex<Phactory<Platform>>>,
}

impl<Platform: pal::Platform> RpcService<Platform> {
    pub fn new(platform: Platform) -> RpcService<Platform> {
        RpcService {
            phactory: Arc::new(Mutex::new(Phactory::new(platform))),
        }
    }
}

impl<Platform> RpcService<Platform>
where
    Platform: pal::Platform + Serialize + DeserializeOwned,
{
    pub fn dispatch_request(
        &self,
        path: String,
        data: &[u8],
    ) -> impl Future<Output = (u16, Vec<u8>)> {
        use prpc::server::{Error, ProtoError};
        let data = data.to_vec();

        let mut server = PhactoryApiServer::new(self.clone());

        async move {
            info!("Dispatching request: {}", path);

            let (code, data) = match server.dispatch_request(&path, data).await {
                Ok(data) => (200, data),
                Err(err) => {
                    error!("Rpc error: {:?}", err);
                    let (code, err) = match err {
                        Error::NotFound => (404, ProtoError::new("Method Not Found")),
                        Error::DecodeError(err) => {
                            (400, ProtoError::new(format!("DecodeError({err:?})")))
                        }
                        Error::AppError(msg) => (500, ProtoError::new(msg)),
                        Error::ContractQueryError(msg) => (500, ProtoError::new(msg)),
                    };
                    (code, prpc::codec::encode_message_to_vec(&err))
                }
            };
            (code, data)
        }
    }
}

impl<Platform: pal::Platform> RpcService<Platform> {
    pub fn lock_phactory(&self) -> MutexGuard<'_, Phactory<Platform>> {
        self.phactory.lock().unwrap()
    }
}

fn create_attestation_report_on<Platform: pal::Platform>(
    platform: &Platform,
    attestation_provider: Option<AttestationProvider>,
    data: &[u8],
) -> RpcResult<pb::Attestation> {
    let encoded_report = match platform.create_attestation_report(attestation_provider, data) {
        Ok(r) => r,
        Err(e) => {
            let message = format!("Failed to create attestation report: {e:?}");
            error!("{}", message);
            return Err(from_display(message));
        }
    };
    Ok(pb::Attestation {
        version: 1,
        provider: serde_json::to_string(&attestation_provider).unwrap(),
        payload: None,
        encoded_report,
        timestamp: now(),
    })
}

#[async_trait::async_trait]
/// A server that process all RPCs.
impl<Platform: pal::Platform + Serialize + DeserializeOwned> PhactoryApi for RpcService<Platform> {
    /// Get basic information about Phactory state.
    async fn get_info(&mut self, _request: ()) -> RpcResult<pb::PhactoryInfo> {
        Ok(self.lock_phactory().get_info())
    }

    /// Sync the parent chain header
    async fn sync_header(&mut self, request: pb::HeadersToSync) -> RpcResult<pb::SyncedTo> {
        let headers = request.decode_headers()?;
        let authority_set_change = request.decode_authority_set_change()?;
        self.lock_phactory()
            .sync_header(headers, authority_set_change)
    }

    /// Sync the parachain header
    async fn sync_para_header(
        &mut self,
        request: pb::ParaHeadersToSync,
    ) -> RpcResult<pb::SyncedTo> {
        let headers = request.decode_headers()?;
        self.lock_phactory()
            .sync_para_header(headers, request.proof)
    }

    async fn sync_combined_headers(
        &mut self,
        request: pb::CombinedHeadersToSync,
    ) -> Result<pb::HeadersSyncedTo, prpc::server::Error> {
        self.lock_phactory().sync_combined_headers(
            request.decode_relaychain_headers()?,
            request.decode_authority_set_change()?,
            request.decode_parachain_headers()?,
            request.proof,
        )
    }

    /// Dispatch blocks (Sync storage changes)"
    async fn dispatch_blocks(&mut self, request: pb::Blocks) -> RpcResult<pb::SyncedTo> {
        let blocks = request.decode_blocks()?;
        self.lock_phactory().dispatch_block(blocks)
    }

    async fn init_runtime(
        &mut self,
        request: pb::InitRuntimeRequest,
    ) -> RpcResult<pb::InitRuntimeResponse> {
        self.lock_phactory().init_runtime(
            request.is_parachain,
            request.decode_genesis_info()?,
            request.decode_genesis_state()?,
            request.decode_operator()?,
            request.debug_set_key.clone(),
            request.decode_attestation_provider()?,
        )
    }

    async fn get_runtime_info(
        &mut self,
        req: pb::GetRuntimeInfoRequest,
    ) -> RpcResult<pb::InitRuntimeResponse> {
        self.lock_phactory()
            .get_runtime_info(req.force_refresh_ra, req.decode_operator()?)
    }

    async fn get_egress_messages(&mut self, _: ()) -> RpcResult<pb::GetEgressMessagesResponse> {
        self.lock_phactory()
            .get_egress_messages()
            .map(pb::GetEgressMessagesResponse::new)
    }

    async fn contract_query(
        &mut self,
        request: pb::ContractQueryRequest,
    ) -> RpcResult<pb::ContractQueryResponse> {
        let (tx, rx) = channel();
        let phactory = self.phactory.clone();
        tokio::spawn(async move {
            if let Ok((cluster_id, effects)) = rx.await {
                phactory
                    .lock()
                    .unwrap()
                    .apply_side_effects(cluster_id, effects);
            }
        });
        let query_fut = self.lock_phactory().contract_query(request, tx)?;
        query_fut.await
    }

    async fn get_worker_state(
        &mut self,
        request: pb::GetWorkerStateRequest,
    ) -> RpcResult<pb::WorkerState> {
        let mut phactory = self.lock_phactory();
        let system = phactory.system()?;
        let gk = system
            .gatekeeper
            .as_ref()
            .ok_or_else(|| from_display("Not a gatekeeper"))?;
        let pubkey: WorkerPublicKey = request
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| from_display("Bad public key"))?;
        let state = gk
            .computing_economics
            .worker_state(&pubkey)
            .ok_or_else(|| from_display("Worker not found"))?;
        Ok(state)
    }

    async fn add_endpoint(
        &mut self,
        request: pb::AddEndpointRequest,
    ) -> RpcResult<pb::GetEndpointResponse> {
        self.lock_phactory()
            .add_endpoint(request.decode_endpoint_type()?, request.endpoint)
    }

    async fn refresh_endpoint_signing_time(&mut self, _: ()) -> RpcResult<pb::GetEndpointResponse> {
        self.lock_phactory().sign_endpoints()
    }

    async fn get_endpoint_info(&mut self, _: ()) -> RpcResult<pb::GetEndpointResponse> {
        self.lock_phactory().get_endpoint_info()
    }

    async fn sign_endpoint_info(
        &mut self,
        request: pb::SignEndpointsRequest,
    ) -> Result<pb::GetEndpointResponse, prpc::server::Error> {
        self.lock_phactory()
            .sign_endpoint_info(VersionedWorkerEndpoints::V1(request.decode_endpoints()?))
    }

    async fn derive_phala_i2p_key(&mut self, _: ()) -> RpcResult<pb::DerivePhalaI2pKeyResponse> {
        let mut phactory = self.lock_phactory();
        let system = phactory.system()?;
        let derive_key = system
            .identity_key
            .derive_sr25519_pair(&[b"PhalaI2PKey"])
            .expect("should not fail with valid key");
        Ok(pb::DerivePhalaI2pKeyResponse {
            phala_i2p_key: derive_key.dump_secret_key().to_vec(),
        })
    }

    async fn echo(&mut self, request: pb::EchoMessage) -> RpcResult<pb::EchoMessage> {
        let echo_msg = request.echo_msg;
        Ok(pb::EchoMessage { echo_msg })
    }

    // WorkerKey Handover Server

    async fn handover_create_challenge(
        &mut self,
        _request: (),
    ) -> RpcResult<pb::HandoverChallenge> {
        let mut phactory = self.lock_phactory();
        let system = phactory.system()?;
        let challenge = system.get_worker_key_challenge();
        Ok(pb::HandoverChallenge::new(challenge))
    }

    async fn handover_start(
        &mut self,
        request: pb::HandoverChallengeResponse,
    ) -> RpcResult<pb::HandoverWorkerKey> {
        let mut phactory = self.lock_phactory();
        let attestation_provider = phactory.attestation_provider;
        let dev_mode = phactory.dev_mode;
        let in_sgx = attestation_provider == Some(AttestationProvider::Ias);
        let system = phactory.system()?;
        let my_identity_key = system.identity_key.clone();

        // 1. verify RA report
        // this also ensure the message integrity
        let challenge_handler = request.decode_challenge_handler().map_err(from_display)?;
        let block_sec = system.now_ms / 1000;
        let block_number = system.block_number;
        let attestation = if dev_mode || !in_sgx {
            info!("Skip RA report check in dev mode");
            None
        } else {
            let payload_hash = sp_core::hashing::blake2_256(&challenge_handler.encode());
            let raw_attestation = request
                .attestation
                .ok_or_else(|| from_display("Attestation not found"))?;
            let attn_to_validate =
                AttestationReport::decode(&mut &raw_attestation.encoded_report[..])
                    .map_err(|_| anyhow!("Decode attestation payload failed"))
                    .unwrap();
            // The time from attestation report is generated by IAS, thus trusted. By default, it's valid for 10h.
            // By ensuring our system timestamp is within the valid period, we know that this pRuntime is not hold back by
            // malicious workers.
            validate_attestation_report(
                Some(attn_to_validate.clone()),
                &payload_hash,
                block_sec,
                false,
                vec![],
                false,
            )
            .map_err(|_| from_display("Invalid RA report from client"))?;
            Some(attn_to_validate)
        };
        // 2. verify challenge validity to prevent replay attack
        let challenge = challenge_handler.challenge;
        if !system.verify_worker_key_challenge(&challenge) {
            return Err(from_display("Invalid challenge"));
        }
        // 3. verify sgx local attestation report to ensure the handover pRuntimes are on the same machine
        if !dev_mode && in_sgx {
            let recv_local_report =
                unsafe { sgx_api_lite::decode(&challenge_handler.sgx_local_report).unwrap() };
            sgx_api_lite::verify(recv_local_report)
                .map_err(|_| from_display("No remote handover"))?;
        } else {
            info!("Skip pRuntime local attestation check in dev mode");
        }
        // 4. verify challenge block height and report timestamp
        // only challenge within 150 blocks (30 minutes) is accepted
        let challenge_height = challenge.block_number;
        if !(challenge_height <= block_number && block_number - challenge_height <= 150) {
            return Err(from_display("Outdated challenge"));
        }
        // 5. verify pruntime launch date
        if !dev_mode && in_sgx {
            let runtime_info = phactory
                .runtime_info
                .as_ref()
                .ok_or_else(|| from_display("Runtime not initialized"))?;
            let my_attn = runtime_info
                .attestation
                .as_ref()
                .ok_or_else(|| from_display("My attestation not found"))?;
            let my_attn_report = my_attn
                .payload
                .as_ref()
                .ok_or_else(|| from_display("My RA report not found"))?;
            let (my_ias_fields, _) = IasFields::from_ias_report(my_attn_report.report.as_bytes())
                .map_err(|_| from_display("Invalid RA report from client"))?;
            let my_mrenclave = my_ias_fields.extend_mrenclave();
            let runtime_state = phactory.runtime_state()?;
            let my_runtime_timestamp = runtime_state
                .chain_storage
                .get_pruntime_added_at(&my_mrenclave)
                .ok_or_else(|| from_display("Key handover not supported in this pRuntime"))?;

            let attestation = attestation.ok_or_else(|| from_display("Attestation not found"))?;
            let mrenclave = match attestation {
                AttestationReport::SgxIas {
                    ra_report,
                    signature: _,
                    raw_signing_cert: _,
                } => {
                    let (ias_fields, _) = IasFields::from_ias_report(&ra_report)
                        .map_err(|_| from_display("Invalid received RA report"))?;
                    ias_fields.extend_mrenclave()
                }
            };
            let req_runtime_timestamp = runtime_state
                .chain_storage
                .get_pruntime_added_at(&mrenclave)
                .ok_or_else(|| from_display("Unknown target pRuntime version"))?;
            // ATTENTION.shelven: relax this check for easy testing and restore it for release
            if my_runtime_timestamp >= req_runtime_timestamp {
                return Err(from_display("No handover for old pRuntime"));
            }
        } else {
            info!("Skip pRuntime timestamp check in dev mode");
        }

        // Share the key with attestation
        let ecdh_pubkey = challenge_handler.ecdh_pubkey;
        let iv = crate::generate_random_iv();
        let (ecdh_pubkey, encrypted_key) = key_share::encrypt_secret_to(
            &my_identity_key,
            &[b"worker_key_handover"],
            &ecdh_pubkey.0,
            &my_identity_key.dump_secret_key(),
            &iv,
        )
        .map_err(from_debug)?;
        let encrypted_key = EncryptedKey {
            ecdh_pubkey: sr25519::Public(ecdh_pubkey),
            encrypted_key,
            iv,
        };
        let runtime_state = phactory.runtime_state()?;
        let genesis_block_hash = runtime_state.genesis_block_hash;
        let encrypted_worker_key = EncryptedWorkerKey {
            genesis_block_hash,
            para_id: runtime_state.para_id,
            dev_mode,
            encrypted_key,
        };

        let worker_key_hash = sp_core::hashing::blake2_256(&encrypted_worker_key.encode());
        let attestation = if dev_mode || !in_sgx {
            info!("Omit RA report in workerkey response in dev mode");
            None
        } else {
            Some(create_attestation_report_on(
                &phactory.platform,
                attestation_provider,
                &worker_key_hash,
            )?)
        };

        Ok(pb::HandoverWorkerKey::new(
            encrypted_worker_key,
            attestation,
        ))
    }

    // WorkerKey Handover Client

    async fn handover_accept_challenge(
        &mut self,
        request: pb::HandoverChallenge,
    ) -> RpcResult<pb::HandoverChallengeResponse> {
        let mut phactory = self.lock_phactory();

        // generate and save tmp key only for key handover encryption
        let handover_key = crate::new_sr25519_key();
        let handover_ecdh_key = handover_key
            .derive_ecdh_key()
            .expect("should never fail with valid key; qed.");
        let ecdh_pubkey = phala_types::EcdhPublicKey(handover_ecdh_key.public());
        phactory.handover_ecdh_key = Some(handover_ecdh_key);

        let challenge = request.decode_challenge().map_err(from_display)?;
        let attestation_provider = phactory.attestation_provider;

        let dev_mode = challenge.dev_mode;
        let in_sgx = attestation_provider == Some(AttestationProvider::Ias);

        // generate local attestation report to ensure the handover pRuntimes are on the same machine
        let sgx_local_report = if dev_mode || !in_sgx {
            vec![]
        } else {
            let its_target_info =
                unsafe { sgx_api_lite::decode(&challenge.sgx_target_info).unwrap() };
            // the report data does not matter since we only care about the origin
            let report = sgx_api_lite::report(its_target_info, &[0; 64]).unwrap();
            sgx_api_lite::encode(&report).to_vec()
        };

        let challenge_handler = ChallengeHandlerInfo {
            challenge,
            sgx_local_report,
            ecdh_pubkey,
        };
        let handler_hash = sp_core::hashing::blake2_256(&challenge_handler.encode());
        let attestation = if dev_mode || !in_sgx {
            info!("Omit RA report in challenge response in dev mode");
            None
        } else {
            Some(create_attestation_report_on(
                &phactory.platform,
                attestation_provider,
                &handler_hash,
            )?)
        };

        Ok(pb::HandoverChallengeResponse::new(
            challenge_handler,
            attestation,
        ))
    }

    async fn handover_receive(&mut self, request: pb::HandoverWorkerKey) -> RpcResult<()> {
        let mut phactory = self.lock_phactory();
        let attestation_provider = phactory.attestation_provider;
        let encrypted_worker_key = request.decode_worker_key().map_err(from_display)?;

        let dev_mode = encrypted_worker_key.dev_mode;
        let in_sgx = attestation_provider == Some(AttestationProvider::Ias);
        // verify RA report
        if !dev_mode && in_sgx {
            let worker_key_hash = sp_core::hashing::blake2_256(&encrypted_worker_key.encode());
            let raw_attestation = request
                .attestation
                .ok_or_else(|| from_display("Attestation not found"))?;
            let attn_to_validate =
                AttestationReport::decode(&mut &raw_attestation.encoded_report[..])
                    .map_err(|_| anyhow!("Decode attestation payload failed"))
                    .unwrap();
            validate_attestation_report(
                Some(attn_to_validate),
                &worker_key_hash,
                now(),
                false,
                vec![],
                false,
            )
            .map_err(|_| from_display("Invalid RA report from server"))?;
        } else {
            info!("Skip RA report check in dev mode");
        }

        let encrypted_key = encrypted_worker_key.encrypted_key;
        let my_ecdh_key = phactory
            .handover_ecdh_key
            .as_ref()
            .ok_or_else(|| from_display("Ecdh key not initialized"))?;
        let secret = key_share::decrypt_secret_from(
            my_ecdh_key,
            &encrypted_key.ecdh_pubkey.0,
            &encrypted_key.encrypted_key,
            &encrypted_key.iv,
        )
        .map_err(from_debug)?;

        // only seal if the key is successfully updated
        phactory
            .save_runtime_data(
                encrypted_worker_key.genesis_block_hash,
                encrypted_worker_key.para_id,
                sr25519::Pair::restore_from_secret_key(&secret),
                false, // we are not sure whether this key is injected
                dev_mode,
            )
            .map_err(from_display)?;

        // clear cached RA report and handover ecdh key to prevent replay
        phactory.runtime_info = None;
        phactory.handover_ecdh_key = None;
        Ok(())
    }

    async fn config_network(
        &mut self,
        request: super::NetworkConfig,
    ) -> Result<(), prpc::server::Error> {
        self.lock_phactory().set_netconfig(request);
        Ok(())
    }

    async fn http_fetch(
        &mut self,
        request: pb::HttpRequest,
    ) -> Result<pb::HttpResponse, prpc::server::Error> {
        use reqwest::{
            header::{HeaderMap, HeaderName, HeaderValue},
            Method,
        };
        use reqwest_env_proxy::EnvProxyBuilder;

        let url: reqwest::Url = request.url.parse().map_err(from_debug)?;

        let client = reqwest::Client::builder()
            .env_proxy(url.host_str().unwrap_or_default())
            .build()
            .map_err(from_debug)?;

        let method: Method = FromStr::from_str(&request.method)
            .or(Err("Invalid HTTP method"))
            .map_err(from_display)?;
        let mut headers = HeaderMap::new();
        for header in &request.headers {
            let name = HeaderName::from_str(&header.name)
                .or(Err("Invalid HTTP header key"))
                .map_err(from_display)?;
            let value = HeaderValue::from_str(&header.value)
                .or(Err("Invalid HTTP header value"))
                .map_err(from_display)?;
            headers.insert(name, value);
        }

        let response = client
            .request(method, url)
            .headers(headers)
            .body(request.body)
            .send()
            .await
            .map_err(from_debug)?;

        let headers: Vec<_> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                let name = name.to_string();
                let value = value.to_str().unwrap_or_default().into();
                pb::HttpHeader { name, value }
            })
            .collect();

        let status_code = response.status().as_u16() as _;
        let body = response
            .bytes()
            .await
            .or(Err("Failed to copy response body"))
            .map_err(from_display)?
            .to_vec();

        Ok(pb::HttpResponse {
            status_code,
            body,
            headers,
        })
    }

    async fn get_contract_info(
        &mut self,
        request: pb::GetContractInfoRequest,
    ) -> Result<pb::GetContractInfoResponse, prpc::server::Error> {
        self.lock_phactory()
            .get_contract_info(&request.contract_ids)
    }

    async fn get_cluster_info(
        &mut self,
        _request: (),
    ) -> Result<pb::GetClusterInfoResponse, prpc::server::Error> {
        self.lock_phactory().get_cluster_info()
    }

    async fn upload_sidevm_code(
        &mut self,
        request: pb::SidevmCode,
    ) -> Result<(), prpc::server::Error> {
        let contract_id: [u8; 32] = request
            .contract
            .try_into()
            .map_err(|_| from_display("Invalid contract id"))?;
        self.lock_phactory()
            .upload_sidevm_code(contract_id.into(), request.code)
    }

    async fn calculate_contract_id(
        &mut self,
        req: pb::ContractParameters,
    ) -> Result<pb::ContractId, prpc::server::Error> {
        let deployer =
            try_decode_hex(&req.deployer).map_err(|_| from_display("Invalid deployer"))?;
        let code_hash =
            try_decode_hex(&req.code_hash).map_err(|_| from_display("Invalid code hash"))?;
        let cluster_id =
            try_decode_hex(&req.cluster_id).map_err(|_| from_display("Invalid cluster id"))?;
        let salt = try_decode_hex(&req.salt).map_err(|_| from_display("Invalid code salt"))?;
        let buf = contract_id_preimage(&deployer, &code_hash, &cluster_id, &salt);
        let hash = sp_core::blake2_256(&buf);
        Ok(pb::ContractId { id: hex(hash) })
    }

    async fn get_network_config(
        &mut self,
        _request: (),
    ) -> Result<pb::NetworkConfigResponse, prpc::server::Error> {
        let phactory = self.lock_phactory();
        Ok(pb::NetworkConfigResponse {
            public_rpc_port: phactory.args.public_port.map(Into::into),
            config: phactory.netconfig.clone(),
        })
    }
    async fn load_chain_state(
        &mut self,
        request: pb::ChainState,
    ) -> Result<(), prpc::server::Error> {
        self.lock_phactory()
            .load_chain_state(request.block_number, request.decode_state()?)
            .map_err(from_display)
    }
    async fn stop(&mut self, request: pb::StopOptions) -> Result<(), prpc::server::Error> {
        self.lock_phactory().stop(request.remove_checkpoints)
    }
}

fn try_decode_hex(hex_str: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(hex_str.strip_prefix("0x").unwrap_or(hex_str))
}
