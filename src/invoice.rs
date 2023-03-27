// RGB wallet library for smart contracts on Bitcoin & Lightning network
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2019-2023 by
//     Dr Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2019-2023 LNP/BP Standards Association. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::num::ParseIntError;
use std::str::FromStr;

use bitcoin::{Address, Network};
use bp::Chain;
use fluent_uri::Uri;
use indexmap::IndexMap;
use rgb::{AttachId, ContractId, SecretSeal};
use strict_encoding::{InvalidIdent, TypeName};

#[derive(Clone, Eq, PartialEq, Hash, Debug, Display)]
pub enum RgbTransport {
    #[display("rgb-rpc{tls}://host/")]
    JsonRpc { tls: bool, host: String },
    #[display("rgb+http{tls}://host/")]
    RestHttp { tls: bool, host: String },
    #[display("rgb+ws{tls}://host/")]
    WebSockets { tls: bool, host: String },
    #[display("rgb+storm://_/")]
    Storm {/* todo */},
    #[display("rgb:")]
    UnspecifiedMeans,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Display)]
pub enum InvoiceState {
    #[display("")]
    Void,
    #[display("{0}.{1}")]
    Fungible(u64, u64),
    #[display("...")] // TODO
    Data(Vec<u8> /* StrictVal */),
    #[display(inner)]
    Attach(AttachId),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Display, From)]
#[display(inner)]
pub enum Beneficiary {
    #[from]
    BlindedSeal(SecretSeal),
    #[from]
    WitnessUtxo(Address),
    // TODO: add BifrostNode(),
}

#[derive(Clone, Eq, PartialEq, Debug, Display)]
// TODO: Change to custom display impl providing support for optionals & query
#[display("{transport}{contract}/{iface}/{value}+{beneficiary}")]
pub struct RgbInvoice {
    pub transport: RgbTransport,
    pub contract: ContractId,
    pub iface: TypeName,
    pub operation: Option<TypeName>,
    pub assignment: Option<TypeName>,
    // pub owned_state: Option<String>,
    pub beneficiary: Beneficiary,
    pub value: u64, // TODO: Change to TypedState
    pub chain: Option<Chain>,
    pub unknown_query: IndexMap<String, String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Display, Error, From)]
#[display(inner)]
pub enum InvoiceParseError {
    #[from]
    Uri(fluent_uri::ParseError),

    #[display(doc_comments)]
    /// invalid invoice.
    Invalid,

    #[from]
    Id(baid58::Baid58ParseError),

    #[display(doc_comments)]
    /// can't recognize beneficiary "": it should be either a bitcoin address or
    /// a blinded UTXO seal.
    Beneficiary(String),

    #[display(doc_comments)]
    /// network {0} is not supported.
    UnsupportedNetwork(Network),

    #[from]
    Num(ParseIntError),

    #[from]
    #[display(doc_comments)]
    /// invalid interface name.
    IfaceName(InvalidIdent),
}

impl FromStr for RgbInvoice {
    type Err = InvoiceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = Uri::parse(s)?;

        let path = uri
            .path()
            .segments()
            .map(|e| e.to_string())
            .collect::<Vec<String>>();

        let mut chain = None;

        let mut assignment = path[2].split('@');
        let (seal, value) = match (assignment.next(), assignment.next()) {
            (Some(a), Some(b)) => {
                let value = u64::from_str_radix(a, 10)?;
                let benefactor = match (SecretSeal::from_str(b), Address::from_str(b)) {
                    (Ok(seal), Err(_)) => Beneficiary::BlindedSeal(seal),
                    (Err(_), Ok(addr)) => {
                        chain = Some(match addr.network {
                            Network::Bitcoin => Chain::Bitcoin,
                            Network::Testnet => Chain::Testnet3,
                            Network::Signet => Chain::Signet,
                            Network::Regtest => Chain::Regtest,
                            unknown => return Err(InvoiceParseError::UnsupportedNetwork(unknown)),
                        });
                        Beneficiary::WitnessUtxo(addr.assume_checked())
                    }
                    (Err(_), Err(_)) => return Err(InvoiceParseError::Beneficiary(a.to_owned())),
                    (Ok(_), Ok(_)) => panic!(
                        "found a string which is both valid bitcoin address and UTXO blind seal"
                    ),
                };
                (benefactor, value)
            }
            _ => return Err(InvoiceParseError::Invalid),
        };

        Ok(RgbInvoice {
            transport: RgbTransport::UnspecifiedMeans,
            contract: ContractId::from_str(&path[0])?,
            iface: TypeName::try_from(path[1].clone())?,
            operation: None,
            assignment: None,
            beneficiary: seal,
            value,
            chain,
            unknown_query: Default::default(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        RgbInvoice::from_str(
            "rgb:EKkb7TMfbPxzn7UhvXqhoCutzdZkSZCNYxVAVjsA67fW/RGB20/100@\
             6kzbKKffP6xftkxn9UP8gWqiC41W16wYKE5CYaVhmEve",
        )
        .unwrap();
    }
}
