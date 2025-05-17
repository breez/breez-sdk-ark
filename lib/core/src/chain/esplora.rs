use ark_client::{error::IntoError, Blockchain, Error, ExplorerUtxo, SpendStatus};
use bitcoin::{Address, Amount, OutPoint, Transaction, Txid};
use esplora_client::Builder;
use std::sync::Arc;

pub struct EsploraBlockchain {
    client: Arc<esplora_client::BlockingClient>,
}

impl EsploraBlockchain {
    pub fn new(url: String) -> Result<Self, Error> {
        let client = Builder::new(&url).build_blocking();

        Ok(Self {
            client: Arc::new(client),
        })
    }
}

impl Blockchain for EsploraBlockchain {
    async fn find_outpoints(&self, address: &Address) -> Result<Vec<ExplorerUtxo>, Error> {
        let script_pubkey = address.script_pubkey();
        let txs = self.client.scripthash_txs(&script_pubkey, None).unwrap();

        let outputs = txs
            .into_iter()
            .flat_map(|tx| {
                let txid = tx.txid;

                let confirmation_blocktime = tx.status.block_time;

                tx.vout
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.scriptpubkey == script_pubkey)
                    .map(|(i, v)| ExplorerUtxo {
                        outpoint: OutPoint {
                            txid,
                            vout: i as u32,
                        },
                        amount: Amount::from_sat(v.value),
                        confirmation_blocktime,
                        // Assume the output is unspent until we dig deeper, further down.
                        is_spent: false,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut utxos = Vec::new();
        for output in outputs.iter() {
            let outpoint = output.outpoint;
            let status = self
                .client
                .get_output_status(&outpoint.txid, outpoint.vout as u64)
                .unwrap();

            match status {
                Some(esplora_client::OutputStatus { spent: false, .. }) | None => {
                    utxos.push(*output);
                }
                Some(esplora_client::OutputStatus { spent: true, .. }) => {
                    utxos.push(ExplorerUtxo {
                        is_spent: true,
                        ..*output
                    })
                }
            }
        }

        Ok(utxos)
    }

    async fn find_tx(&self, txid: &Txid) -> Result<Option<Transaction>, Error> {
        let tx = self
            .client
            .get_tx(txid)
            .map_err(|e| e.to_string().into_error())?;

        Ok(tx)
    }

    async fn get_output_status(&self, txid: &Txid, vout: u32) -> Result<SpendStatus, Error> {
        let status = self
            .client
            .get_output_status(txid, vout as u64)
            .map_err(|e| e.to_string().into_error())?;

        Ok(SpendStatus {
            spend_txid: status.and_then(|s| s.txid),
        })
    }

    async fn broadcast(&self, tx: &Transaction) -> Result<(), Error> {
        self.client
            .broadcast(tx)
            .map_err(|e| e.to_string().into_error())?;

        Ok(())
    }
}
