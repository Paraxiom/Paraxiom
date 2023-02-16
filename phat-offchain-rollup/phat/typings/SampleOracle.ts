import type * as PhalaSdk from "@phala/sdk";
import type * as DevPhase from "devphase";
import type * as DPT from "devphase/etc/typings";
import type { ContractCallResult, ContractQuery } from "@polkadot/api-contract/base/types";
import type { ContractCallOutcome, ContractOptions } from "@polkadot/api-contract/types";
import type { Codec } from "@polkadot/types/types";

export namespace SampleOracle {
    type InkEnv_Types_AccountId = any;
    type PrimitiveTypes_H160 = any;
    type SampleOracle_SampleOracle_Error = { BadOrigin: null } | { NotConfigurated: null } | { BadAbi: null } | { FailedToGetStorage: null } | { FailedToDecodeStorage: null } | { FailedToEstimateGas: null } | { FailedToCreateRollupSession: null } | { FailedToFetchLock: null } | { FailedToReadQueueHead: null };
    type Result = { Ok: Option } | { Err: number[] };
    type PhatOffchainRollup_Raw = any;
    type PhatOffchainRollup_Cond = { Eq: [PhatOffchainRollup_Raw, PhatOffchainRollup_Raw] };
    type Option = { None: null } | { Some: PhatOffchainRollup_RollupResult };
    type PhatOffchainRollup_RollupTx = { conds: PhatOffchainRollup_Cond[], actions: PhatOffchainRollup_Raw[], updates: [ PhatOffchainRollup_Raw, Option ][] };
    type PhatOffchainRollup_RollupResult = { tx: PhatOffchainRollup_RollupTx, signature: Option, target: Option };

    /** */
    /** Queries */
    /** */
    namespace ContractQuery {
        export interface Owner extends DPT.ContractQuery {
            (certificateData: PhalaSdk.CertificateData, options: ContractOptions): DPT.CallResult<DPT.CallOutcome<DPT.IJson<InkEnv_Types_AccountId>>>;
        }

        export interface RollupHandler_HandleRollup extends DPT.ContractQuery {
            (certificateData: PhalaSdk.CertificateData, options: ContractOptions): DPT.CallResult<DPT.CallOutcome<DPT.IJson<Result>>>;
        }
    }

    export interface MapMessageQuery extends DPT.MapMessageQuery {
        owner: ContractQuery.Owner;
        'rollupHandler::handleRollup': ContractQuery.RollupHandler_HandleRollup;
    }

    /** */
    /** Transactions */
    /** */
    namespace ContractTx {
        export interface Config extends DPT.ContractTx {
            (options: ContractOptions, rpc: string, anchor: PrimitiveTypes_H160): DPT.SubmittableExtrinsic;
        }
    }

    export interface MapMessageTx extends DPT.MapMessageTx {
        config: ContractTx.Config;
    }

    /** */
    /** Contract */
    /** */
    export declare class Contract extends DPT.Contract {
        get query(): MapMessageQuery;
        get tx(): MapMessageTx;
    }

    /** */
    /** Contract factory */
    /** */
    export declare class Factory extends DevPhase.ContractFactory {
        instantiate<T = Contract>(constructor: "default", params: never[], options?: DevPhase.InstantiateOptions): Promise<T>;
    }
}
