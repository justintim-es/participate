use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use participate_runtime_api::ParticipateApi as ParticipateStorageApi;
use sp_std::vec::Vec;

#[rpc]
pub trait ParticipateApi<Hash> {
    #[rpc(name = "participate_living_hashes")]
    fn get_living_hashes(&self) -> Result<Vec<Hash>>;

}
pub struct LivingHashes<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}
impl<C, M> LivingHashes<C, M> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}
impl<C, Block> ParticipateApi<<Block as BlockT>::Hash> for LivingHashes<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: ParticipateStorageApi<Block>,
{
	fn get_living_hashes(&self) -> Result<Vec<<Block as BlockT>::Hash>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(self.client.info().best_hash);

		let runtime_api_result = api.get_living_hashes(&at);
		runtime_api_result.map_err(|e| RpcError {
			code: ErrorCode::ServerError(9876), // No real reason for this value
			message: "Something wrong".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
